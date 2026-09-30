use rusqlite::Connection;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifiedUser {
    pub id: String,
    pub username: String,
    pub nama: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifyTokenResult {
    pub user: VerifiedUser,
    pub api_key: String,
}

pub async fn verify_token_with_server(
    api_url: &str,
    api_key: &str,
    token: &str,
) -> Result<VerifyTokenResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Gagal inisialisasi HTTP client: {}", e))?;

    let url = format!("{}/api/v1/auth/verify-token", api_url.trim_end_matches('/'));

    let mut req = client.post(&url)
        .json(&serde_json::json!({ "token": token }));

    if !api_key.trim().is_empty() {
        req = req.header("X-API-Key", api_key.trim());
    }

    let res = req.send()
        .await
        .map_err(|e| format!("Gagal menghubungi peladen autentikasi: {}", e))?;

    let status = res.status();
    let body: Value = res.json().await.map_err(|e| format!("Respon server tidak valid: {}", e))?;

    if !status.is_success() || !body["success"].as_bool().unwrap_or(false) {
        let msg = body["message"].as_str().unwrap_or("Verifikasi token gagal");
        return Err(msg.to_string());
    }

    let user_val = &body["user"];
    let user: VerifiedUser = serde_json::from_value(user_val.clone())
        .map_err(|e| format!("Format data profil tidak sesuai: {}", e))?;

    let returned_api_key = body["api_key"].as_str().unwrap_or("").to_string();

    Ok(VerifyTokenResult { user, api_key: returned_api_key })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExchangeCodeResult {
    pub user: VerifiedUser,
    pub token: String,
    pub api_key: String,
}

pub async fn exchange_code_with_server(
    api_url: &str,
    api_key: &str,
    code: &str,
) -> Result<ExchangeCodeResult, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Gagal inisialisasi HTTP client: {}", e))?;

    let url = format!("{}/api/v1/auth/exchange-code", api_url.trim_end_matches('/'));

    let mut req = client.post(&url)
        .json(&serde_json::json!({ "code": code.trim() }));

    if !api_key.trim().is_empty() {
        req = req.header("X-API-Key", api_key.trim());
    }

    let res = req.send()
        .await
        .map_err(|e| format!("Gagal menghubungi peladen autentikasi: {}", e))?;

    let status = res.status();
    let body: Value = res.json().await.map_err(|e| format!("Respon server tidak valid: {}", e))?;

    if !status.is_success() || !body["success"].as_bool().unwrap_or(false) {
        let msg = body["message"].as_str().unwrap_or("Verifikasi kode otorisasi gagal");
        return Err(msg.to_string());
    }

    let user_val = &body["user"];
    let user: VerifiedUser = serde_json::from_value(user_val.clone())
        .map_err(|e| format!("Format data profil tidak sesuai: {}", e))?;

    let token = body["token"].as_str().unwrap_or("").to_string();
    if token.is_empty() {
        return Err("Peladen tidak mengembalikan token autentikasi.".to_string());
    }

    let returned_api_key = body["api_key"].as_str().unwrap_or("").to_string();

    Ok(ExchangeCodeResult { user, token, api_key: returned_api_key })
}

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

    // Hapus record yang terhapus di server (tombstones)
    if let Some(deleted_map) = payload.get("deleted").and_then(|v| v.as_object()) {
        for (table_name, ids_val) in deleted_map {
            if let Some(ids_arr) = ids_val.as_array() {
                for id_val in ids_arr {
                    if let Some(id_str) = id_val.as_str() {
                        let del_query = format!("DELETE FROM `{}` WHERE id = ?;", table_name);
                        let _ = conn.execute(&del_query, [id_str]);
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
) -> Result<HashMap<String, Vec<String>>, String> {
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

    let mut synced_ids_map = HashMap::new();
    if let Some(synced_obj) = response_json.get("synced_ids").and_then(|v| v.as_object()) {
        for (tbl, ids_val) in synced_obj {
            if let Some(ids_arr) = ids_val.as_array() {
                let ids: Vec<String> = ids_arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                synced_ids_map.insert(tbl.clone(), ids);
            }
        }
    } else {
        // Fallback jika server tidak mengembalikan synced_ids
        for (table, records) in payload_map {
            let ids: Vec<String> = records.iter()
                .filter_map(|r| r.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect();
            synced_ids_map.insert(table.clone(), ids);
        }
    }

    Ok(synced_ids_map)
}

// Fungsi sinkron untuk memperbarui status lokal record yang terunggah menjadi 'synced'
pub fn mark_pushed_records_synced(
    conn: &Connection,
    synced_ids_map: &HashMap<String, Vec<String>>,
) -> Result<(), String> {
    for (table, ids) in synced_ids_map {
        for id_val in ids {
            conn.execute(
                &format!("UPDATE `{}` SET sync_status = 'synced' WHERE id = ?;", table),
                [id_val],
            ).map_err(|e| format!("Gagal meng-update sync_status untuk {} ID {}: {}", table, id_val, e))?;
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

    #[test]
    fn test_apply_pull_payload_tombstones() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE settings (id TEXT PRIMARY KEY, key TEXT UNIQUE, value TEXT);", []).unwrap();
        conn.execute("CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT, sync_status TEXT DEFAULT 'synced');", []).unwrap();
        conn.execute("INSERT INTO users (id, name) VALUES ('usr-to-delete', 'Siswa Dihapus');", []).unwrap();

        let payload = serde_json::json!({
            "success": true,
            "timestamp": "2026-08-05 14:00:00",
            "data": {},
            "deleted": {
                "users": ["usr-to-delete"]
            }
        });

        let result = apply_pull_payload(&conn, &payload);
        assert!(result.is_ok());

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE id = 'usr-to-delete';", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "Record yang tercantum di tombstones harus terhapus dari SQLite");
    }
}
