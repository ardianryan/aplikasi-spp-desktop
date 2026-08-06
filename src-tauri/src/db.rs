use rusqlite::Connection;
use std::fs;
use uuid::Uuid;
use tauri::AppHandle;
use tauri::Manager;

pub struct DbState {
    pub conn: std::sync::Mutex<Connection>,
}

/// Derive a stable SQLCipher passphrase from the stored API key.
/// Falls back to a device-unique seed if API key is not yet set.
fn load_cipher_key(db_path: &std::path::Path) -> String {
    // Try to read the api_key from the DB in plain mode first (bootstrap scenario).
    // In normal operation we derive it from a local file in the app config dir.
    let key_file = db_path.parent()
        .map(|p| p.join(".cipher_seed"))
        .unwrap_or_else(|| std::path::PathBuf::from(".cipher_seed"));

    if let Ok(key) = fs::read_to_string(&key_file) {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    // First run: generate a random seed, persist it
    let seed = format!("psk_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let _ = fs::write(&key_file, &seed);
    seed
}

/// Update the cipher seed when the API key is saved for the first time.
pub fn update_cipher_seed(app_handle: &AppHandle, api_key: &str) {
    if let Ok(dir) = app_handle.path().app_data_dir() {
        let key_file = dir.join(".cipher_seed");
        // Derive key = first 40 chars of api_key (already high-entropy)
        let derived = if api_key.len() >= 20 {
            api_key[..api_key.len().min(64)].to_string()
        } else {
            // pad with existing seed if api key too short
            api_key.to_string()
        };
        let _ = fs::write(key_file, derived);
    }
}

pub fn init_db(app_handle: &AppHandle) -> Result<Connection, String> {
    // Get the standard app data directory
    let mut db_path = app_handle.path().app_data_dir()
        .map_err(|e| format!("Gagal mendapatkan app data dir: {}", e))?;
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&db_path)
        .map_err(|e| format!("Gagal membuat direktori data: {}", e))?;
    
    db_path.push("partisipasi.db");
    println!("Database path: {:?}", db_path);
    
    // Load SQLCipher passphrase
    let cipher_key = load_cipher_key(&db_path);

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Gagal membuka database SQLite: {}", e))?;

    // Activate SQLCipher encryption
    conn.execute_batch(&format!("PRAGMA key = '{}';", cipher_key.replace("'", "''")))
        .map_err(|e| format!("Gagal mengaktifkan SQLCipher key: {}", e))?;
        
    // Enable foreign keys and test read access
    if let Err(e) = conn.execute("PRAGMA foreign_keys = ON;", []) {
        let err_str = e.to_string();
        if err_str.contains("file is not a database") || err_str.contains("NOTADB") {
            println!("DB lama tidak terenkripsi atau corrupt. Membuat ulang database terenkripsi baru...");
            drop(conn);
            let _ = fs::remove_file(&db_path);
            
            let conn_new = Connection::open(&db_path)
                .map_err(|e| format!("Gagal membuka database SQLite baru: {}", e))?;
            conn_new.execute_batch(&format!("PRAGMA key = '{}';", cipher_key.replace("'", "''")))
                .map_err(|e| format!("Gagal mengaktifkan SQLCipher key baru: {}", e))?;
            conn_new.execute("PRAGMA foreign_keys = ON;", [])
                .map_err(|e| format!("Gagal mengaktifkan PRAGMA foreign_keys: {}", e))?;
            return Ok(conn_new);
        }
        return Err(format!("Gagal mengaktifkan PRAGMA foreign_keys: {}", e));
    }
        
    Ok(conn)
}

fn clean_mysql_for_sqlite(sql: &str) -> String {
    let mut cleaned = String::new();
    
    // Bersihkan komentar terlebih dahulu
    let sql_without_comments = sql.lines()
        .map(|line| {
            let t = line.trim();
            if t.starts_with("--") || t.starts_with("#") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<&str>>()
        .join("\n");
        
    // Split berdasarkan semicolon untuk mengambil setiap statement SQL
    for statement in sql_without_comments.split(';') {
        let stmt_trimmed = statement.trim();
        if stmt_trimmed.is_empty() {
            continue;
        }
        
        if stmt_trimmed.to_uppercase().starts_with("CREATE TABLE") {
            // Pemrosesan CREATE TABLE
            let mut processed = stmt_trimmed.to_string();
            
            // Ubah ENUM(...) menjadi TEXT
            while let Some(start_enum) = processed.find("ENUM(") {
                if let Some(end_enum) = processed[start_enum..].find(')') {
                    let full_enum = &processed[start_enum..start_enum + end_enum + 1];
                    processed = processed.replace(full_enum, "TEXT");
                } else {
                    break;
                }
            }
            
            // Ubah tipe data MySQL ke SQLite
            processed = processed.replace("TINYINT(1)", "INTEGER");
            processed = processed.replace("ON UPDATE CURRENT_TIMESTAMP", "");
            
            // Hapus komentar kolom MySQL
            while let Some(start_comment) = processed.find("COMMENT '") {
                if let Some(end_comment) = processed[start_comment + 9..].find('\'') {
                    let full_comment = &processed[start_comment..start_comment + 9 + end_comment + 1];
                    processed = processed.replace(full_comment, "");
                } else {
                    break;
                }
            }
            
            // Proses baris-per-baris dalam blok CREATE TABLE
            let mut lines: Vec<String> = Vec::new();
            for l in processed.lines() {
                let l_trimmed = l.trim();
                if l_trimmed.is_empty() {
                    continue;
                }
                
                // Lewati indeks non-unik (tidak krusial di SQLite lokal)
                if l_trimmed.starts_with("KEY ") || l_trimmed.starts_with("KEY` ") || l_trimmed.starts_with("KEY `") {
                    continue;
                }
                
                // Ubah UNIQUE KEY menjadi UNIQUE constraint
                if l_trimmed.starts_with("UNIQUE KEY ") {
                    if let Some(idx) = l_trimmed.find('(') {
                        let cols = &l_trimmed[idx..];
                        lines.push(format!("    UNIQUE {}", cols));
                        continue;
                    }
                }
                
                // Hapus deklarasi ENGINE MySQL di baris penutup tabel
                if l_trimmed.contains(") ENGINE=") {
                    lines.push(");".to_string());
                    continue;
                }
                
                // Relaksasi constraint: Hapus NOT NULL agar kolom yang di-ALTER menjadi nullable nantinya aman dari error SQLite
                let mut clean_line = l.to_string();
                if clean_line.contains("NOT NULL") && !clean_line.contains("PRIMARY KEY") && !clean_line.contains("`id` ") {
                    clean_line = clean_line.replace("NOT NULL", "");
                }
                
                lines.push(clean_line);
            }
            
            let mut table_sql = lines.join("\n");
            
            // Hapus koma gantung di bagian akhir kolom jika ada baris terhapus sebelumnya
            if let Some(close_idx) = table_sql.rfind(')') {
                let mut last_comma_idx = None;
                for (i, c) in table_sql[..close_idx].char_indices().rev() {
                    if c == ',' {
                        last_comma_idx = Some(i);
                        break;
                    } else if !c.is_whitespace() && c != '\n' && c != '\r' {
                        break;
                    }
                }
                if let Some(idx) = last_comma_idx {
                    table_sql.remove(idx);
                }
            }
            
            cleaned.push_str(&table_sql);
            cleaned.push_str(";\n");
            
        } else if stmt_trimmed.to_uppercase().starts_with("ALTER TABLE") {
            // Pemrosesan ALTER TABLE MySQL agar kompatibel dengan keterbatasan SQLite
            let parts: Vec<&str> = stmt_trimmed.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let table_name = parts[2].trim_matches('`');
            
            // Ambil semua perintah tindakan (bisa berisi beberapa ADD COLUMN dipisah koma)
            let start_pos = stmt_trimmed.find(parts[2]).unwrap() + parts[2].len();
            let actions_str = stmt_trimmed[start_pos..].trim();
            
            let mut actions = Vec::new();
            let mut current_action = String::new();
            let mut paren_count = 0;
            
            // Split berdasarkan koma dengan memperhatikan tanda kurung
            for c in actions_str.chars() {
                if c == '(' {
                    paren_count += 1;
                } else if c == ')' {
                    paren_count -= 1;
                }
                
                if c == ',' && paren_count == 0 {
                    actions.push(current_action.trim().to_string());
                    current_action.clear();
                } else {
                    current_action.push(c);
                }
            }
            if !current_action.trim().is_empty() {
                actions.push(current_action.trim().to_string());
            }
            
            for action in actions {
                let action_upper = action.to_uppercase();
                let act_trimmed = action.trim();
                
                if act_trimmed.starts_with("ADD COLUMN") || act_trimmed.starts_with("ADD ") {
                    // Pastikan bukan penambahan constraint key/foreign key
                    if action_upper.contains("KEY ") || action_upper.contains("CONSTRAINT ") || action_upper.contains("FOREIGN KEY ") {
                        if action_upper.contains("UNIQUE KEY") || action_upper.contains("UNIQUE ") {
                            if let Some(start_idx) = act_trimmed.find('(') {
                                if let Some(end_idx) = act_trimmed[start_idx..].find(')') {
                                    let cols = &act_trimmed[start_idx..start_idx + end_idx + 1];
                                    let key_name = act_trimmed.split_whitespace().nth(3).unwrap_or("idx").trim_matches('`');
                                    cleaned.push_str(&format!(
                                        "CREATE UNIQUE INDEX IF NOT EXISTS `{}` ON `{}` {};\n",
                                        key_name, table_name, cols
                                    ));
                                }
                            }
                        } else if action_upper.contains("KEY ") {
                            if let Some(start_idx) = act_trimmed.find('(') {
                                if let Some(end_idx) = act_trimmed[start_idx..].find(')') {
                                    let cols = &act_trimmed[start_idx..start_idx + end_idx + 1];
                                    let key_name = act_trimmed.split_whitespace().nth(2).unwrap_or("idx").trim_matches('`');
                                    cleaned.push_str(&format!(
                                        "CREATE INDEX IF NOT EXISTS `{}` ON `{}` {};\n",
                                        key_name, table_name, cols
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                    
                    // Eksekusi penambahan kolom murni
                    let mut col_def = act_trimmed.to_string();
                    if col_def.starts_with("ADD COLUMN ") {
                        col_def = col_def["ADD COLUMN ".len()..].to_string();
                    } else if col_def.starts_with("ADD ") {
                        col_def = col_def["ADD ".len()..].to_string();
                    }
                    
                    // Hilangkan klausa AFTER karena SQLite tidak mendukungnya
                    if let Some(after_idx) = col_def.to_uppercase().find(" AFTER ") {
                        col_def = col_def[..after_idx].to_string();
                    }
                    
                    // Bersihkan tipe data dan constraint
                    col_def = col_def.replace("TINYINT(1)", "INTEGER");
                    col_def = col_def.replace("NOT NULL", "");
                    
                    while let Some(start_enum) = col_def.find("ENUM(") {
                        if let Some(end_enum) = col_def[start_enum..].find(')') {
                            let full_enum = &col_def[start_enum..start_enum + end_enum + 1];
                            col_def = col_def.replace(full_enum, "TEXT");
                        } else {
                            break;
                        }
                    }
                    
                    cleaned.push_str(&format!(
                        "ALTER TABLE `{}` ADD COLUMN {};\n",
                        table_name, col_def
                    ));
                }
                // MODIFY COLUMN, DROP FOREIGN KEY, dan tindakan unsupported lainnya dilewati (no-op di SQLite)
            }
        } else {
            // Query reguler seperti INSERT / UPDATE
            let mut processed = stmt_trimmed.to_string();
            processed = processed.replace("NOW()", "CURRENT_TIMESTAMP");
            
            // Terjemahkan UPDATE JOIN MySQL ke SQLite subquery
            let normalized = processed.replace("\n", " ").replace("\r", " ");
            let normalized_upper = normalized.to_uppercase();
            if normalized_upper.contains("UPDATE `RELIEFS` R") && normalized_upper.contains("JOIN `STUDENTS` S") {
                processed = "UPDATE `reliefs` SET `academic_year_id` = (SELECT `academic_year_id` FROM `students` WHERE `students`.`id` = `reliefs`.`student_id`)".to_string();
            }
            
            cleaned.push_str(&processed);
            cleaned.push_str(";\n");
        }
    }
    
    // Ganti pemanggilan UUID() menjadi ID statis di SQLite
    let mut final_sql = cleaned;
    while let Some(idx) = final_sql.find("UUID()") {
        let new_uuid = format!("'{}'", Uuid::new_v4());
        final_sql.replace_range(idx..idx + 6, &new_uuid);
    }
    
    final_sql
}

pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Check if migrations table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            run_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    ).map_err(|e| format!("Gagal membuat tabel migrations: {}", e))?;

    // Load static migrations from migrations.rs
    use crate::migrations::MIGRATIONS;

    for &(name, sql_content) in MIGRATIONS {
        // Check if migration already ran
        let already_run: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?);",
            [name],
            |row| row.get(0),
        ).unwrap_or(false);

        if !already_run {
            println!("Running migration: {}", name);
            let cleaned_sql = clean_mysql_for_sqlite(sql_content);
            
            // Execute statements
            // rusqlite's execute_batch can run multiple statements separated by semicolon
            conn.execute_batch(&cleaned_sql)
                .map_err(|e| format!("Gagal menjalankan migrasi {} ke SQLite:\nError: {}\nSQL:\n{}", name, e, cleaned_sql))?;

            conn.execute(
                "INSERT INTO _migrations (name) VALUES (?);",
                [name],
            ).map_err(|e| format!("Gagal mencatat status migrasi: {}", e))?;
        }
    }

    // Post-migration: Inject sync_status into transaction tables if not exists
    let transaction_tables = vec![
        "payments",
        "payment_details",
        "savings_transactions",
        "journals",
        "savings",
        "reliefs",
    ];

    for table in transaction_tables {
        // Check if sync_status column exists in the table
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({});", table))
            .map_err(|e| format!("Gagal pragma info: {}", e))?;
        
        let mut rows = stmt.query([])
            .map_err(|e| format!("Gagal query pragma: {}", e))?;
            
        let mut sync_status_exists = false;
        while let Some(row) = rows.next().map_err(|e| format!("Gagal fetch pragma: {}", e))? {
            let col_name: String = row.get(1).unwrap_or_default();
            if col_name == "sync_status" {
                sync_status_exists = true;
                break;
            }
        }
        
        if !sync_status_exists {
            println!("Adding sync_status to table: {}", table);
            let alter_query = format!("ALTER TABLE {} ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'synced';", table);
            conn.execute(&alter_query, [])
                .map_err(|e| format!("Gagal menambahkan sync_status ke {}: {}", table, e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_mysql_for_sqlite() {
        let mysql_ddl = "CREATE TABLE `test` (\n  `id` varchar(36) NOT NULL,\n  `status` ENUM('active','inactive') NOT NULL,\n  `is_active` TINYINT(1) DEFAULT 1\n) ENGINE=InnoDB COMMENT 'table comment';";
        let cleaned = clean_mysql_for_sqlite(mysql_ddl);

        assert!(!cleaned.contains("ENUM("));
        assert!(!cleaned.contains("TINYINT(1)"));
        assert!(cleaned.contains("TEXT"));
        assert!(cleaned.contains("INTEGER"));
    }

    #[test]
    fn test_run_migrations_in_memory() {
        let conn = Connection::open_in_memory().expect("Gagal membuat in-memory DB");
        let result = run_migrations(&conn);
        assert!(result.is_ok(), "Migrasi SQLite in-memory harus berhasil: {:?}", result.err());

        // Verifikasi tabel _migrations terisi
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM _migrations;", [], |r| r.get(0)).unwrap();
        assert!(count > 0, "Tabel _migrations harus mencatat migrasi");
    }
}
