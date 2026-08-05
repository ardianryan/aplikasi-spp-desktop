use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;

pub fn get_sync_settings(conn: &Connection) -> (String, String) {
    let api_url: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'api_url';",
        [],
        |row| row.get(0),
    ).unwrap_or_default();

    let api_key: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'api_key';",
        [],
        |row| row.get(0),
    ).unwrap_or_default();

    (api_url, api_key)
}

pub fn set_sync_settings(conn: &Connection, api_url: &str, api_key: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_url', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
        [api_url],
    ).map_err(|e| format!("Gagal menyimpan api_url: {}", e))?;

    conn.execute(
        "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_key', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
        [api_key],
    ).map_err(|e| format!("Gagal menyimpan api_key: {}", e))?;

    Ok(())
}

fn upsert_json_record(conn: &Connection, table_name: &str, record: &Value) -> Result<(), String> {
    let obj = record.as_object().ok_or("Record bukan object JSON")?;
    if obj.is_empty() {
        return Ok(());
    }

    let mut has_sync_status = false;
    if let Ok(mut stmt) = conn.prepare(&format!("PRAGMA table_info({});", table_name)) {
        if let Ok(mut rows) = stmt.query([]) {
            while let Ok(Some(row)) = rows.next() {
                let name: String = row.get(1).unwrap_or_default();
                if name == "sync_status" {
                    has_sync_status = true;
                    break;
                }
            }
        }
    }

    if has_sync_status {
        if let Some(id_val) = obj.get("id").and_then(|v| v.as_str()) {
            let local_status: Option<String> = conn.query_row(
                &format!("SELECT sync_status FROM {} WHERE id = ?;", table_name),
                [id_val],
                |row| row.get(0)
            ).ok();
            if let Some(status) = local_status {
                if status == "pending_push" {
                    return Ok(());
                }
            }
        }
    }

    let mut columns = Vec::new();
    let mut place_holders = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    for (k, v) in obj {
        columns.push(format!("`{}`", k));
        place_holders.push("?".to_string());

        let val: Box<dyn rusqlite::ToSql> = match v {
            Value::Null => Box::new(rusqlite::types::Null),
            Value::Bool(b) => Box::new(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Box::new(i)
                } else if let Some(f) = n.as_f64() {
                    Box::new(f)
                } else {
                    Box::new(rusqlite::types::Null)
                }
            }
            Value::String(s) => Box::new(s.clone()),
            _ => Box::new(v.to_string()),
        };
        values.push(val);
    }

    let columns_str = columns.join(", ");
    let placeholders_str = place_holders.join(", ");

    // INSERT OR REPLACE menangani semua unique constraint secara otomatis
    // tanpa perlu mengetahui kolom mana yang menjadi conflict target
    let query = format!(
        "INSERT OR REPLACE INTO `{}` ({}) VALUES ({});",
        table_name, columns_str, placeholders_str
    );

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|b| b.as_ref()).collect();
    conn.execute(&query, params.as_slice())
        .map_err(|e| format!("Gagal upsert ke tabel {}: {}", table_name, e))?;

    Ok(())
}

fn get_pending_records(conn: &Connection, table_name: &str) -> Result<Vec<Value>, String> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM `{}` WHERE sync_status = 'pending_push';", table_name))
        .map_err(|e| format!("Gagal mempersiapkan query push untuk {}: {}", table_name, e))?;

    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or_default().to_string())
        .collect();

    let mut rows = stmt.query([])
        .map_err(|e| format!("Gagal mengeksekusi query push untuk {}: {}", table_name, e))?;

    let mut records = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("Gagal membaca baris: {}", e))? {
        let mut map = serde_json::Map::new();
        for i in 0..col_count {
            let name = &col_names[i];
            if name == "sync_status" {
                continue;
            }

            let val = match row.get_ref(i).unwrap() {
                rusqlite::types::ValueRef::Null => Value::Null,
                rusqlite::types::ValueRef::Integer(val) => Value::Number(serde_json::Number::from(val)),
                rusqlite::types::ValueRef::Real(val) => Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| serde_json::Number::from(0))),
                rusqlite::types::ValueRef::Text(bytes) => {
                    let s = std::str::from_utf8(bytes).unwrap_or_default();
                    Value::String(s.to_string())
                }
                rusqlite::types::ValueRef::Blob(bytes) => {
                    let s = std::str::from_utf8(bytes).unwrap_or_default();
                    Value::String(s.to_string())
                }
            };
            map.insert(name.clone(), val);
        }
        records.push(Value::Object(map));
    }

    Ok(records)
}

// -------------------------------------------------------------
// BAGIAN 1: SINKRONISASI PULL (Async Download & Sync Upsert)
// -------------------------------------------------------------

// Fungsi async murni untuk mengunduh payload dari server (Tanpa menahan koneksi SQLite)
pub async fn fetch_pull_payload(api_url: &str, api_key: &str, last_synced: &str) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Gagal menginisialisasi HTTP client: {}", e))?;

    let url = format!("{}/api/v1/sync/pull?since={}", api_url.trim_end_matches('/'), urlencoding::encode(last_synced));
    println!("Pulling data from: {}", url);

    let res = client.get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("Koneksi gagal saat pull: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Server pull mengembalikan status error: {}", res.status()));
    }

    let payload: Value = res.json()
        .await
        .map_err(|e| format!("Gagal mem-parsing JSON response pull: {}", e))?;

    if !payload["success"].as_bool().unwrap_or(false) {
        return Err(format!("Pull gagal di server: {}", payload["message"].as_str().unwrap_or("Unknown error")));
    }

    Ok(payload)
}

// Fungsi sinkron murni untuk melakukan upsert data payload hasil unduhan ke SQLite
pub fn apply_pull_payload(conn: &Connection, payload: &Value) -> Result<String, String> {
    let sync_time = payload["timestamp"].as_str().unwrap_or("").to_string();
    let data = &payload["data"];

    if let Some(tables_map) = data.as_object() {
        for (table_name, records) in tables_map {
            if let Some(arr) = records.as_array() {
                if !arr.is_empty() {
                    println!("Tabel {}: memproses {} baris", table_name, arr.len());
                    for record in arr {
                        upsert_json_record(conn, table_name, record)?;
                    }
                }
            }
        }
    }

    // Simpan timestamp sinkronisasi terakhir
    if !sync_time.is_empty() {
        conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'last_synced_at', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [&sync_time],
        ).map_err(|e| format!("Gagal menyimpan last_synced_at: {}", e))?;
    }

    Ok(sync_time)
}

// -------------------------------------------------------------
// BAGIAN 2: SINKRONISASI PUSH (Sync Read, Async Upload, Sync Mark)
// -------------------------------------------------------------

// Fungsi sinkron untuk membaca semua data transaksi lokal yang pending_push
pub fn get_all_pending_records(conn: &Connection) -> Result<(HashMap<String, Vec<Value>>, usize), String> {
    let transaction_tables = vec![
        "payments",
        "payment_details",
        "savings_transactions",
        "journals",
        "savings",
        "reliefs",
    ];

    let mut payload_map = HashMap::new();
    let mut total_pending = 0;

    for table in transaction_tables {
        let records = get_pending_records(conn, table)?;
        if !records.is_empty() {
            total_pending += records.len();
            payload_map.insert(table.to_string(), records);
        }
    }

    Ok((payload_map, total_pending))
}

// Fungsi async murni untuk mengunggah payload transaksi ke server online
pub async fn upload_push_payload(
    api_url: &str,
    api_key: &str,
    payload_map: &HashMap<String, Vec<Value>>,
    total_pending: usize,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Gagal menginisialisasi HTTP client: {}", e))?;

    let url = format!("{}/api/v1/sync/push", api_url.trim_end_matches('/'));
    println!("Pushing data ({}) to: {}", total_pending, url);

    let res = client.post(&url)
        .header("X-API-Key", api_key)
        .json(payload_map)
        .send()
        .await
        .map_err(|e| format!("Koneksi gagal saat push: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Server push mengembalikan status error: {}", res.status()));
    }

    let response_json: Value = res.json()
        .await
        .map_err(|e| format!("Gagal mem-parsing JSON response push: {}", e))?;

    if !response_json["success"].as_bool().unwrap_or(false) {
        return Err(format!("Push gagal di server: {}", response_json["message"].as_str().unwrap_or("Unknown error")));
    }

    Ok(())
}

// Fungsi sinkron untuk memperbarui status lokal record yang terunggah menjadi 'synced'
pub fn mark_pushed_records_synced(conn: &Connection, payload_map: &HashMap<String, Vec<Value>>) -> Result<(), String> {
    for (table, records) in payload_map {
        for record in records {
            if let Some(id_val) = record["id"].as_str() {
                conn.execute(
                    &format!("UPDATE `{}` SET sync_status = 'synced' WHERE id = ?;", table),
                    [id_val],
                ).map_err(|e| format!("Gagal meng-update sync_status untuk {} ID {}: {}", table, id_val, e))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_settings_get_set() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE settings (id TEXT PRIMARY KEY, key TEXT UNIQUE, value TEXT);", []).unwrap();

        set_sync_settings(&conn, "https://partisipasi.sch.id", "psk_test_12345").unwrap();
        let (url, key) = get_sync_settings(&conn);

        assert_eq!(url, "https://partisipasi.sch.id");
        assert_eq!(key, "psk_test_12345");
    }

    #[test]
    fn test_apply_pull_payload_upsert() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE settings (id TEXT PRIMARY KEY, key TEXT UNIQUE, value TEXT);", []).unwrap();
        conn.execute("CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT, sync_status TEXT DEFAULT 'synced');", []).unwrap();

        let payload = serde_json::json!({
            "success": true,
            "timestamp": "2026-08-05 13:47:00",
            "data": {
                "users": [
                    {"id": "usr-1", "name": "Budi Kasir"}
                ]
            }
        });

        let result = apply_pull_payload(&conn, &payload);
        assert!(result.is_ok());

        let name: String = conn.query_row("SELECT name FROM users WHERE id = 'usr-1';", [], |r| r.get(0)).unwrap();
        assert_eq!(name, "Budi Kasir");

        let last_synced: String = conn.query_row("SELECT value FROM settings WHERE key = 'last_synced_at';", [], |r| r.get(0)).unwrap();
        assert_eq!(last_synced, "2026-08-05 13:47:00");
    }
}
