use tauri::State;
use rusqlite::params;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::Local;
use sha2::{Sha256, Digest};

use crate::PendingDeepLink;
use crate::db::DbState;
use crate::sync::{
    get_sync_settings, set_sync_settings, fetch_pull_payload, apply_pull_payload,
    get_all_pending_records, upload_push_payload, mark_pushed_records_synced,
    verify_token_with_server
};

// Structs untuk transfer data ke Frontend
#[derive(Serialize, Deserialize, Clone)]
pub struct StudentInfo {
    pub id: String,
    pub nis: Option<String>,
    pub nisn: Option<String>,
    pub nama: String,
    pub kelas: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StudentDetails {
    pub student: StudentInfo,
    pub summary: BillSummary,
    pub assignments: Vec<PaymentAssignment>,
    pub payments: Vec<PaymentHistory>,
    pub savings_balance: f64,
    pub unpaid_years: Vec<String>,
    pub active_year_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct BillSummary {
    pub total_tagihan: f64,
    pub total_dibayar: f64,
    pub total_keringanan: f64,
    pub total_tunggakan: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaymentAssignment {
    pub id: String,
    pub payment_type_name: String,
    pub payment_type_type: String, // bulanan / bebas
    pub month: Option<i32>,
    pub month_name: Option<String>,
    pub amount: f64,
    pub paid_amount: f64,
    pub relief_amount: f64,
    pub remaining_amount: f64,
    pub status: String, // lunas / belum_lunas / cicilan
    pub academic_year_id: Option<String>,
    pub academic_year_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaymentHistory {
    pub id: String,
    pub transaction_code: String,
    pub total_amount: f64,
    pub payment_method_name: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaymentHistoryItem {
    pub id: String,
    pub transaction_code: String,
    pub student_name: String,
    pub student_class: String,
    pub total_amount: f64,
    pub payment_method_name: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub sync_status: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PaymentItemInput {
    pub payment_assignment_id: String,
    pub amount: f64,
    pub payment_type_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReceiptData {
    pub transaction_code: String,
    pub created_at: String,
    pub payment_method_name: String,
    pub total_amount: f64,
    pub notes: Option<String>,
    pub student_nama: String,
    pub student_nis: Option<String>,
    pub student_nisn: Option<String>,
    pub student_kelas: Option<String>,
    pub items: Vec<ReceiptItem>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReceiptItem {
    pub payment_type_name: String,
    pub payment_type_type: String,
    pub month: Option<i32>,
    pub month_name: Option<String>,
    pub amount: f64,
    pub academic_year_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavingsInfo {
    pub student_id: String,
    pub balance: f64,
    pub transactions: Vec<SavingsTransaction>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavingsTransaction {
    pub id: String,
    pub transaction_type: String, // deposit / withdraw
    pub amount: f64,
    pub notes: Option<String>,
    pub created_at: String,
}

// 1. Pencarian Siswa (POS)
#[tauri::command]
pub fn search_students(query: String, status: String, state: State<'_, DbState>) -> Result<Vec<StudentInfo>, String> {
    let conn = state.conn.lock().unwrap();
    let q = format!("%{}%", query);
    let is_active_val = if status == "inactive" { 0 } else { 1 };
    
    let mut stmt = conn.prepare(
        "SELECT s.id, s.nis, s.nisn, s.nama, c.name as kelas 
         FROM students s
         LEFT JOIN classrooms c ON s.classroom_id = c.id
         WHERE (s.nama LIKE ? OR s.nis LIKE ? OR s.nisn LIKE ?)
           AND s.is_active = ?
         LIMIT 20;"
    ).map_err(|e| format!("Query error: {}", e))?;
    
    let rows = stmt.query_map(params![q, q, q, is_active_val], |row| {
        Ok(StudentInfo {
            id: row.get(0)?,
            nis: row.get(1)?,
            nisn: row.get(2)?,
            nama: row.get(3)?,
            kelas: row.get(4)?,
        })
    }).map_err(|e| format!("Fetch error: {}", e))?;
    
    let mut list = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            list.push(item);
        }
    }
    
    Ok(list)
}

// 2. Detail Pembayaran & Buku Tabungan Siswa
#[tauri::command]
pub fn get_student_details(student_id: String, state: State<'_, DbState>) -> Result<StudentDetails, String> {
    let conn = state.conn.lock().unwrap();
    
    // a. Info Siswa
    let student = conn.query_row(
        "SELECT s.id, s.nis, s.nisn, s.nama, c.name as kelas 
         FROM students s
         LEFT JOIN classrooms c ON s.classroom_id = c.id
         WHERE s.id = ?;",
        [&student_id],
        |row| {
            Ok(StudentInfo {
                id: row.get(0)?,
                nis: row.get(1)?,
                nisn: row.get(2)?,
                nama: row.get(3)?,
                kelas: row.get(4)?,
            })
        }
    ).map_err(|e| format!("Siswa tidak ditemukan: {}", e))?;
    
    // b. Tagihan / Assignments
    let mut stmt = conn.prepare(
        "SELECT a.id, t.name, t.type, a.month, a.amount, a.paid_amount, a.relief_amount, a.status, a.academic_year_id, y.name as academic_year_name
         FROM payment_assignments a
         JOIN payment_types t ON a.payment_type_id = t.id
         LEFT JOIN academic_years y ON a.academic_year_id = y.id
         WHERE a.student_id = ?;"
    ).map_err(|e| format!("Tagihan query error: {}", e))?;
    
    let ass_rows = stmt.query_map([&student_id], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let t_type: String = row.get(2)?;
        let month: Option<i32> = row.get(3)?;
        let amount: f64 = row.get(4)?;
        let paid_amount: f64 = row.get(5)?;
        let relief_amount: f64 = row.get(6)?;
        let status: String = row.get(7)?;
        let academic_year_id: Option<String> = row.get(8)?;
        let academic_year_name: Option<String> = row.get(9)?;
        
        let month_name = month.map(|m| match m {
            7 => "Juli".to_string(),
            8 => "Agustus".to_string(),
            9 => "September".to_string(),
            10 => "Oktober".to_string(),
            11 => "November".to_string(),
            12 => "Desember".to_string(),
            1 => "Januari".to_string(),
            2 => "Februari".to_string(),
            3 => "Maret".to_string(),
            4 => "April".to_string(),
            5 => "Mei".to_string(),
            6 => "Juni".to_string(),
            _ => format!("Bulan {}", m),
        });
        
        let remaining_amount = amount - paid_amount - relief_amount;
        
        Ok(PaymentAssignment {
            id,
            payment_type_name: name,
            payment_type_type: t_type,
            month,
            month_name,
            amount,
            paid_amount,
            relief_amount,
            remaining_amount: if remaining_amount < 0.0 { 0.0 } else { remaining_amount },
            status,
            academic_year_id,
            academic_year_name,
        })
    }).map_err(|e| format!("Tagihan fetch error: {}", e))?;
    
    let mut assignments = Vec::new();
    let mut summary = BillSummary::default();
    
    for r in ass_rows {
        if let Ok(item) = r {
            summary.total_tagihan += item.amount;
            summary.total_dibayar += item.paid_amount;
            summary.total_keringanan += item.relief_amount;
            summary.total_tunggakan += item.remaining_amount;
            assignments.push(item);
        }
    }

    // Sort assignments: by academic_year_name DESC, lalu type, lalu name, lalu month
    assignments.sort_by(|a, b| {
        let year_a = a.academic_year_name.as_deref().unwrap_or("");
        let year_b = b.academic_year_name.as_deref().unwrap_or("");
        let cmp = year_b.cmp(year_a); // descending
        if cmp != std::cmp::Ordering::Equal {
            return cmp;
        }

        let type_cmp = a.payment_type_type.cmp(&b.payment_type_type);
        if type_cmp != std::cmp::Ordering::Equal {
            return type_cmp;
        }

        let name_cmp = a.payment_type_name.cmp(&b.payment_type_name);
        if name_cmp != std::cmp::Ordering::Equal {
            return name_cmp;
        }

        let month_a = a.month.unwrap_or(999);
        let month_b = b.month.unwrap_or(999);
        let order_a = if month_a >= 7 { month_a - 6 } else { month_a + 6 };
        let order_b = if month_b >= 7 { month_b - 6 } else { month_b + 6 };

        order_a.cmp(&order_b)
    });
    
    // c. Riwayat Pembayaran
    let mut stmt = conn.prepare(
        "SELECT id, transaction_code, total_amount, payment_method_name, notes, created_at
         FROM payments
         WHERE student_id = ?
         ORDER BY created_at DESC;"
    ).map_err(|e| format!("Riwayat query error: {}", e))?;
    
    let pay_rows = stmt.query_map([&student_id], |row| {
        Ok(PaymentHistory {
            id: row.get(0)?,
            transaction_code: row.get(1)?,
            total_amount: row.get(2)?,
            payment_method_name: row.get(3)?,
            notes: row.get(4)?,
            created_at: row.get(5)?,
        })
    }).map_err(|e| format!("Riwayat fetch error: {}", e))?;
    
    let mut payments = Vec::new();
    for r in pay_rows {
        if let Ok(item) = r {
            payments.push(item);
        }
    }
    
    // d. Saldo Tabungan Siswa
    let savings_balance: f64 = conn.query_row(
        "SELECT balance FROM savings WHERE student_id = ?;",
        [&student_id],
        |row| row.get(0),
    ).unwrap_or(0.0);

    // e. Detect unpaid previous academic years
    let active_year_id: Option<String> = conn.query_row(
        "SELECT id FROM academic_years WHERE is_active = 1 LIMIT 1;",
        [],
        |row| row.get(0),
    ).ok();

    let mut unpaid_years_map = std::collections::BTreeMap::new();
    for a in &assignments {
        if let Some(y_id) = &a.academic_year_id {
            if let Some(active_id) = &active_year_id {
                if y_id != active_id && a.remaining_amount > 0.0 {
                    if let Some(y_name) = &a.academic_year_name {
                        unpaid_years_map.insert(y_id.clone(), y_name.clone());
                    }
                }
            }
        }
    }
    let unpaid_years: Vec<String> = unpaid_years_map.into_values().collect();
    
    Ok(StudentDetails {
        student,
        summary,
        assignments,
        payments,
        savings_balance,
        unpaid_years,
        active_year_id,
    })
}

// 3. Proses Input Transaksi Pembayaran (POS) Offline-First
#[tauri::command]
pub fn save_payment(
    student_id: String,
    academic_year_id: String,
    total_amount: f64,
    payment_method_id: Option<String>,
    payment_method_name: String,
    notes: Option<String>,
    items: Vec<PaymentItemInput>,
    loket_id: String,
    state: State<'_, DbState>,
) -> Result<String, String> {
    let mut conn = state.conn.lock().unwrap();
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi DB: {}", e))?;
    
    // a. Generate Transaction Code Unik
    let date_str = Local::now().format("%Y%m%d").to_string();
    let random_hex = format!("{:x}", Uuid::new_v4().as_u128() % 16777215); // 6 hex digits
    let trx_code = format!("TRX-{}-{}-{}", loket_id.to_uppercase(), date_str, random_hex);
    
    // b. Simpan Header Pembayaran (payments)
    let payment_id = Uuid::new_v4().to_string();
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    tx.execute(
        "INSERT INTO payments (id, transaction_code, student_id, user_id, total_amount, payment_method_id, payment_method_name, notes, academic_year_id, created_at, sync_status) 
         VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, 'pending_push');",
        params![
            payment_id,
            trx_code,
            student_id,
            total_amount,
            payment_method_id,
            payment_method_name,
            notes,
            academic_year_id,
            now_str,
        ]
    ).map_err(|e| format!("Gagal menyimpan header transaksi: {}", e))?;
    
    // c. Simpan Rincian Pembayaran (payment_details) & Update Assignments lokal
    for item in &items {
        let detail_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO payment_details (id, payment_id, payment_assignment_id, amount, created_at, sync_status) 
             VALUES (?, ?, ?, ?, ?, 'pending_push');",
            params![detail_id, payment_id, item.payment_assignment_id, item.amount, now_str]
        ).map_err(|e| format!("Gagal menyimpan detail transaksi: {}", e))?;
        
        // Update nominal terbayar di payment_assignments lokal
        tx.execute(
            "UPDATE payment_assignments 
             SET paid_amount = paid_amount + ?,
                 status = CASE 
                     WHEN (paid_amount + ?) >= (amount - relief_amount) THEN 'lunas'
                     WHEN (paid_amount + ?) > 0 THEN 'cicilan'
                     ELSE 'belum_lunas'
                 END,
                 updated_at = ?
             WHERE id = ?;",
            params![item.amount, item.amount, item.amount, now_str, item.payment_assignment_id]
        ).map_err(|e| format!("Gagal memperbarui status tagihan lokal: {}", e))?;
        
        // d. Catat Buku Jurnal Otomatis (Debet Masuk Kas)
        let journal_id = Uuid::new_v4().to_string();
        let journal_desc = format!("Penerimaan Pembayaran {} - Siswa ID {}", item.payment_type_name, student_id);
        tx.execute(
            "INSERT INTO journals (id, academic_year_id, date, type, category, description, amount, created_at, sync_status) 
             VALUES (?, ?, ?, 'debet', 'pembayaran_spp', ?, ?, ?, 'pending_push');",
            params![
                journal_id,
                academic_year_id,
                &now_str[..10], // YYYY-MM-DD
                journal_desc,
                item.amount,
                now_str,
            ]
        ).map_err(|e| format!("Gagal menyimpan jurnal penerimaan: {}", e))?;
    }
    
    tx.commit().map_err(|e| format!("Gagal commit database: {}", e))?;
    
    Ok(trx_code)
}

// 4. Tabungan Murid (Mutasi Setor / Tarik)
#[tauri::command]
pub fn save_savings_transaction(
    student_id: String,
    academic_year_id: String,
    transaction_type: String, // deposit / withdraw
    amount: f64,
    notes: Option<String>,
    state: State<'_, DbState>,
) -> Result<f64, String> {
    let mut conn = state.conn.lock().unwrap();
    let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi DB: {}", e))?;
    
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    // a. Cek atau Buat Akun Tabungan lokal jika belum ada
    let savings_id: String = tx.query_row(
        "SELECT id FROM savings WHERE student_id = ?;",
        [&student_id],
        |row| row.get(0)
    ).unwrap_or_else(|_| {
        let new_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO savings (id, student_id, balance, created_at, sync_status) 
             VALUES (?, ?, 0.0, ?, 'pending_push');",
            params![new_id, student_id, now_str]
        ).unwrap();
        new_id
    });
    
    // Get current balance
    let current_balance: f64 = tx.query_row(
        "SELECT balance FROM savings WHERE id = ?;",
        [&savings_id],
        |row| row.get(0)
    ).unwrap_or(0.0);
    
    // b. Hitung Saldo Baru & Validasi Penarikan
    let new_balance = match transaction_type.as_str() {
        "deposit" => current_balance + amount,
        "withdraw" => {
            if current_balance < amount {
                return Err("Saldo tabungan tidak mencukupi untuk penarikan!".to_string());
            }
            current_balance - amount
        }
        _ => return Err("Tipe transaksi tabungan tidak valid (wajib deposit/withdraw).".to_string()),
    };
    
    // c. Catat Transaksi Tabungan (savings_transactions)
    let trx_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO savings_transactions (id, savings_id, type, amount, notes, created_at, sync_status) 
         VALUES (?, ?, ?, ?, ?, ?, 'pending_push');",
        params![
            trx_id,
            savings_id,
            transaction_type,
            amount,
            notes,
            now_str,
        ]
    ).map_err(|e| format!("Gagal menyimpan log transaksi tabungan: {}", e))?;
    
    // d. Perbarui Saldo Akun Tabungan
    tx.execute(
        "UPDATE savings SET balance = ?, updated_at = ?, sync_status = 'pending_push' WHERE id = ?;",
        params![new_balance, now_str, savings_id]
    ).map_err(|e| format!("Gagal memperbarui saldo tabungan: {}", e))?;
    
    // e. Log ke Jurnal Buku Kas
    // Setoran tabungan = Debet masuk kas lembaga (dana tabungan)
    // Penarikan tabungan = Kredit keluar kas lembaga
    let journal_id = Uuid::new_v4().to_string();
    let j_type = if transaction_type == "deposit" { "debet" } else { "kredit" };
    let j_desc = format!("Transaksi Tabungan {} - Siswa ID {}", transaction_type, student_id);
    
    tx.execute(
        "INSERT INTO journals (id, academic_year_id, date, type, category, description, amount, created_at, sync_status) 
         VALUES (?, ?, ?, ?, 'tabungan_siswa', ?, ?, ?, 'pending_push');",
        params![
            journal_id,
            academic_year_id,
            &now_str[..10],
            j_type,
            j_desc,
            amount,
            now_str,
        ]
    ).map_err(|e| format!("Gagal mencatat jurnal tabungan: {}", e))?;
    
    tx.commit().map_err(|e| format!("Gagal commit transaksi: {}", e))?;
    
    Ok(new_balance)
}

// 5. Config Sync & manual Sync trigger
#[tauri::command]
pub fn get_sync_config(state: State<'_, DbState>) -> Result<(String, String), String> {
    let conn = state.conn.lock().unwrap();
    Ok(get_sync_settings(&conn))
}

#[tauri::command]
pub fn is_savings_enabled(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().unwrap();
    let val: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'savings_enabled';",
        [],
        |row| row.get(0),
    ).unwrap_or_else(|_| "0".to_string());
    Ok(val == "1")
}

#[tauri::command]
pub fn get_school_settings(state: State<'_, DbState>) -> Result<std::collections::HashMap<String, String>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT key, value FROM settings;").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let k: String = row.get(0)?;
        let v: String = row.get(1)?;
        Ok((k, v))
    }).map_err(|e| e.to_string())?;
    
    let mut map = std::collections::HashMap::new();
    for r in rows {
        if let Ok((k, v)) = r {
            map.insert(k, v);
        }
    }
    Ok(map)
}

#[tauri::command]
pub fn set_sync_config(api_url: String, api_key: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    set_sync_settings(&conn, &api_url, &api_key)?;
    Ok(())
}

/// Returns true if this is a fresh installation (no api_url configured yet)
#[tauri::command]
pub fn is_fresh_install(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().unwrap();
    let api_url: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'api_url';",
        [],
        |row| row.get(0),
    ).unwrap_or_default();
    Ok(api_url.trim().is_empty())
}

#[tauri::command]
pub async fn trigger_sync(state: State<'_, DbState>) -> Result<String, String> {
    // 1. Ambil pengaturan sync & last_synced_at (lock sesingkat mungkin)
    let (api_url, api_key, last_synced_at) = {
        let conn = state.conn.lock().unwrap();
        let (url, key) = get_sync_settings(&conn);
        let last_synced: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'last_synced_at';",
            [],
            |row| row.get(0),
        ).unwrap_or_else(|_| "1970-01-01 00:00:00".to_string());
        (url, key, last_synced)
    };
    
    if api_key.is_empty() {
        return Err("API Key kosong. Mohon konfigurasi koneksi API online terlebih dahulu di pengaturan.".to_string());
    }
    
    // 2. Tahap PULL: Unduh payload dari server (Tanpa menahan lock database)
    let pull_payload = fetch_pull_payload(&api_url, &api_key, &last_synced_at).await?;
    
    // 3. Simpan data PULL ke SQLite (Lock database secara sinkron menggunakan transaksi)
    let sync_time = {
        let mut conn = state.conn.lock().unwrap();
        // Matikan sementara foreign key check untuk bulk insert lintas-tabel
        conn.execute("PRAGMA foreign_keys = OFF;", []).map_err(|e| format!("Gagal menonaktifkan foreign keys: {}", e))?;
        
        let sync_res: Result<String, String> = (|| {
            let tx = conn.transaction().map_err(|e| format!("Gagal memulai transaksi database: {}", e))?;
            let sync_time = apply_pull_payload(&tx, &pull_payload)?;
            tx.commit().map_err(|e| format!("Gagal melakukan commit data sinkronisasi ke SQLite: {}", e))?;
            Ok(sync_time)
        })();

        // Nyalakan kembali foreign key check setelah proses selesai
        let _ = conn.execute("PRAGMA foreign_keys = ON;", []);

        sync_res?
    };
    
    // 4. Tahap PUSH: Baca transaksi pending dari SQLite (Lock database)
    let (pending_payload, total_pending) = {
        let conn = state.conn.lock().unwrap();
        get_all_pending_records(&conn)?
    };
    
    let mut push_result = "Tidak ada data transaksi lokal baru untuk di-push.".to_string();
    if total_pending > 0 {
        // 5. Unggah data PUSH ke server online (Tanpa menahan lock database)
        let synced_ids = upload_push_payload(&api_url, &api_key, &pending_payload, total_pending).await?;
        let total_synced: usize = synced_ids.values().map(|v| v.len()).sum();
        
        // 6. Tandai record yang terkonfirmasi sukses sebagai synced di SQLite (Lock database)
        {
            let conn = state.conn.lock().unwrap();
            mark_pushed_records_synced(&conn, &synced_ids)?;
        }
        push_result = format!("Berhasil mensinkronkan {} dari {} transaksi lokal ke server.", total_synced, total_pending);
    }
    
    Ok(format!("Pull Sukses (Timestamp Server: {}). {}", sync_time, push_result))
}

// 6. Ambil Tahun Akademik Aktif
#[tauri::command]
pub fn get_active_academic_year(state: State<'_, DbState>) -> Result<Option<(String, String)>, String> {
    let conn = state.conn.lock().unwrap();
    let result: Option<(String, String)> = conn.query_row(
        "SELECT id, name FROM academic_years WHERE is_active = 1 LIMIT 1;",
        [],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).ok();
    Ok(result)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserSession {
    pub token: String,
    pub nama: String,
    pub role: String,
}

#[tauri::command]
pub fn get_active_session(state: State<'_, DbState>) -> Result<Option<UserSession>, String> {
    let conn = state.conn.lock().unwrap();
    let res: Option<UserSession> = conn.query_row(
        "SELECT ut.token, u.nama, u.role 
         FROM user_tokens ut
         JOIN users u ON ut.user_id = u.id
         ORDER BY ut.created_at DESC LIMIT 1;",
        [],
        |row| {
            Ok(UserSession {
                token: row.get(0)?,
                nama: row.get(1)?,
                role: row.get(2)?,
            })
        }
    ).ok();
    Ok(res)
}

#[tauri::command]
pub async fn save_session(
    token: String,
    nama: String,
    role: String,
    state: State<'_, DbState>,
) -> Result<UserSession, String> {
    // 1. Ambil sync config
    let (api_url, api_key) = {
        let conn = state.conn.lock().unwrap();
        get_sync_settings(&conn)
    };

    // 2. Jika api_url tersedia, lakukan validasi resmi ke peladen
    let (final_user_id, final_nama, final_role, returned_api_key) = if !api_url.is_empty() {
        match verify_token_with_server(&api_url, &api_key, &token).await {
            Ok(verified) => (Some(verified.user.id), verified.user.nama, verified.user.role, verified.api_key),
            Err(e) => {
                println!("Verifikasi token server gagal/offline: {}", e);
                // Jika offline atau token lokal khusus, periksa apakah token sudah ada di SQLite
                let conn = state.conn.lock().unwrap();
                let local_user: Option<(String, String, String)> = conn.query_row(
                    "SELECT u.id, u.nama, u.role FROM user_tokens ut JOIN users u ON ut.user_id = u.id WHERE ut.token = ? LIMIT 1;",
                    [&token],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                ).ok();

                if let Some((uid, u_nama, u_role)) = local_user {
                    (Some(uid), u_nama, u_role, String::new())
                } else if token.starts_with("offline_") {
                    (None, nama, role, String::new())
                } else {
                    return Err(format!("Autentikasi ditolak oleh peladen: {}", e));
                }
            }
        }
    } else {
        (None, nama, role, String::new())
    };

    let conn = state.conn.lock().unwrap();
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Simpan API Key jika peladen mengembalikannya (Zero-Config provisioning)
    if !returned_api_key.is_empty() {
        conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_key', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [&returned_api_key],
        ).ok();
    }
    
    let user_id = if let Some(uid) = final_user_id {
        // Upsert user resmi ke database SQLite lokal
        conn.execute(
            "INSERT INTO users (id, username, nama, role, is_active, created_at) 
             VALUES (?, ?, ?, ?, 1, ?)
             ON CONFLICT(id) DO UPDATE SET nama = excluded.nama, role = excluded.role, is_active = 1;",
            params![uid, format!("usr_{}", &uid[..8.min(uid.len())]), final_nama, final_role, now_str]
        ).map_err(|e| format!("Gagal menyimpan user: {}", e))?;
        uid
    } else {
        conn.query_row(
            "SELECT id FROM users WHERE nama = ? LIMIT 1;",
            [&final_nama],
            |row| row.get(0)
        ).unwrap_or_else(|_| {
            let new_uid = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO users (id, username, nama, role, is_active, created_at) 
                 VALUES (?, ?, ?, ?, 1, ?);",
                params![new_uid, format!("sso_{}", &Uuid::new_v4().to_string()[..8]), final_nama, final_role, now_str]
            ).unwrap();
            new_uid
        })
    };
    
    let token_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO user_tokens (id, user_id, token, created_at) VALUES (?, ?, ?, ?);",
        params![token_id, user_id, token, now_str]
    ).map_err(|e| format!("Gagal menyimpan token sesi: {}", e))?;
    
    Ok(UserSession {
        token,
        nama: final_nama,
        role: final_role,
    })
}

#[tauri::command]
pub async fn login_with_pairing_code(
    code: String,
    state: State<'_, DbState>,
) -> Result<UserSession, String> {
    let clean_code = code.trim().replace(" ", "").replace("-", "");
    if clean_code.len() < 6 {
        return Err("Kode otorisasi harus terdiri atas 6 digit angka.".to_string());
    }

    let (api_url, api_key) = {
        let conn = state.conn.lock().unwrap();
        get_sync_settings(&conn)
    };

    if api_url.is_empty() {
        return Err("Alamat peladen web sekolah belum dikonfigurasi. Silakan periksa pengaturan sambungan.".to_string());
    }

    let exchange = crate::sync::exchange_code_with_server(&api_url, &api_key, &clean_code).await?;

    let conn = state.conn.lock().unwrap();
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Simpan API Key jika peladen mengembalikannya (Zero-Config provisioning)
    if !exchange.api_key.is_empty() {
        conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_key', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [&exchange.api_key],
        ).ok();
    }

    let uid = exchange.user.id;

    conn.execute(
        "INSERT INTO users (id, username, nama, role, is_active, created_at) 
         VALUES (?, ?, ?, ?, 1, ?)
         ON CONFLICT(id) DO UPDATE SET nama = excluded.nama, role = excluded.role, is_active = 1;",
        params![uid, exchange.user.username, exchange.user.nama, exchange.user.role, now_str]
    ).map_err(|e| format!("Gagal menyimpan profil pengguna: {}", e))?;

    let token_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO user_tokens (id, user_id, token, created_at) VALUES (?, ?, ?, ?);",
        params![token_id, uid, exchange.token, now_str]
    ).map_err(|e| format!("Gagal menyimpan token sesi: {}", e))?;

    Ok(UserSession {
        token: exchange.token,
        nama: exchange.user.nama,
        role: exchange.user.role,
    })
}

#[tauri::command]
pub fn logout(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute("DELETE FROM user_tokens;", [])
        .map_err(|e| format!("Gagal logout: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_pending_deep_link(state: State<'_, PendingDeepLink>) -> Option<String> {
    state.0.lock().unwrap().take()
}

fn hash_pin_sha256(pin: &str) -> String {
    let mut hasher = Sha256::new();
    // Salt domain separation untuk mencegah serangan precomputed rainbow table (v1)
    hasher.update(b"psk_offline_pin_salt_v1:");
    hasher.update(pin.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_pin_with_salt(pin: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"psk_offline_pin_v2:");
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(pin.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
pub fn set_offline_pin(pin: String, state: State<'_, DbState>) -> Result<(), String> {
    let trimmed = pin.trim();
    if trimmed.len() != 6 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err("PIN harus berupa 6 digit angka.".to_string());
    }
    
    let conn = state.conn.lock().unwrap();
    
    // Dapatkan atau buat salt dinamis unik per perangkat
    let salt: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'cashier_offline_pin_salt';",
        [],
        |row| row.get(0)
    ).unwrap_or_else(|_| {
        let new_salt = Uuid::new_v4().to_string().replace("-", "");
        let _ = conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'cashier_offline_pin_salt', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [&new_salt],
        );
        new_salt
    });

    let hashed = hash_pin_with_salt(trimmed, &salt);
    conn.execute(
        "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'cashier_offline_pin', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
        [&hashed],
    ).map_err(|e| format!("Gagal menyimpan PIN offline: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn has_offline_pin(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM settings WHERE key = 'cashier_offline_pin' AND value IS NOT NULL AND length(value) > 0;",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    Ok(count > 0)
}

#[tauri::command]
pub fn verify_offline_pin(pin: String, state: State<'_, DbState>) -> Result<Option<UserSession>, String> {
    let trimmed = pin.trim();
    let conn = state.conn.lock().unwrap();

    let salt: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'cashier_offline_pin_salt';",
        [],
        |row| row.get(0)
    ).unwrap_or_default();

    let hashed_v2 = hash_pin_with_salt(trimmed, &salt);
    let hashed_v1 = hash_pin_sha256(trimmed);

    let stored_hash: Option<String> = conn.query_row(
        "SELECT value FROM settings WHERE key = 'cashier_offline_pin';",
        [],
        |row| row.get(0)
    ).ok();

    if let Some(expected) = stored_hash {
        if expected == hashed_v2 || expected == hashed_v1 {
            // Berhasil cocok, ambil session user aktif terakhir atau default kasir
            let user: Option<(String, String, String)> = conn.query_row(
                "SELECT id, nama, role FROM users WHERE is_active = 1 ORDER BY created_at DESC LIMIT 1;",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            ).ok();

            let (user_id, nama, role) = user.unwrap_or_else(|| (Uuid::new_v4().to_string(), "Kasir Offline".to_string(), "kasir".to_string()));
            let token = format!("offline_pin_{}", &Uuid::new_v4().to_string().replace("-", "")[..16]);
            
            let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let token_id = Uuid::new_v4().to_string();
            let _ = conn.execute(
                "INSERT INTO user_tokens (id, user_id, token, created_at) VALUES (?, ?, ?, ?);",
                params![token_id, user_id, token, now_str]
            );

            return Ok(Some(UserSession {
                token,
                nama,
                role,
            }));
        }
    }

    Ok(None)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TodayStats {
    pub today_total: f64,
    pub monthly_total: f64,
    pub today_count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RecentTransaction {
    pub id: String,
    pub transaction_code: String,
    pub student_name: String,
    pub student_class: String,
    pub total_amount: f64,
    pub created_at: String,
    pub sync_status: String,
}

#[tauri::command]
pub fn get_pending_sync_count(state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    let payments_pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payments WHERE sync_status != 'synced';",
        [],
        |r| r.get(0)
    ).unwrap_or(0);

    let savings_pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM savings_transactions WHERE sync_status != 'synced';",
        [],
        |r| r.get(0)
    ).unwrap_or(0);

    let journals_pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM journals WHERE sync_status != 'synced';",
        [],
        |r| r.get(0)
    ).unwrap_or(0);

    Ok(payments_pending + savings_pending + journals_pending)
}

#[tauri::command]
pub fn get_today_cashier_stats(state: State<'_, DbState>) -> Result<TodayStats, String> {
    let conn = state.conn.lock().unwrap();
    let today_prefix = Local::now().format("%Y-%m-%d").to_string();
    let month_prefix = Local::now().format("%Y-%m").to_string();

    let today_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments WHERE created_at LIKE ?;",
        [format!("{}%", today_prefix)],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let monthly_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments WHERE created_at LIKE ?;",
        [format!("{}%", month_prefix)],
        |row| row.get(0),
    ).unwrap_or(0.0);

    let today_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payments WHERE created_at LIKE ?;",
        [format!("{}%", today_prefix)],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(TodayStats {
        today_total,
        monthly_total,
        today_count,
    })
}

#[tauri::command]
pub fn get_today_payments(state: State<'_, DbState>) -> Result<Vec<RecentTransaction>, String> {
    let conn = state.conn.lock().unwrap();
    let today_prefix = Local::now().format("%Y-%m-%d").to_string();

    let mut stmt = conn.prepare(
        "SELECT p.id, p.transaction_code, s.nama, s.kelas, p.total_amount, p.created_at, p.sync_status
         FROM payments p
         JOIN students s ON p.student_id = s.id
         WHERE p.created_at LIKE ?
         ORDER BY p.created_at DESC;"
    ).map_err(|e| format!("Gagal menyiapkan query: {}", e))?;

    let rows = stmt.query_map([format!("{}%", today_prefix)], |row| {
        Ok(RecentTransaction {
            id: row.get(0)?,
            transaction_code: row.get(1)?,
            student_name: row.get(2)?,
            student_class: row.get(3).unwrap_or_default(),
            total_amount: row.get(4)?,
            created_at: row.get(5)?,
            sync_status: row.get(6)?,
        })
    }).map_err(|e| format!("Gagal menjalankan query: {}", e))?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            list.push(item);
        }
    }
    Ok(list)
}

#[tauri::command]
pub async fn test_sync_connection(api_url: String, api_key: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Gagal inisialisasi HTTP client: {}", e))?;

    let base_url = api_url.trim_end_matches('/');

    // Jika API Key kosong, cukup uji ketersediaan peladen melalui endpoint ping
    if api_key.trim().is_empty() {
        let ping_url = format!("{}/api/v1/ping", base_url);
        let res = client.get(&ping_url)
            .send()
            .await
            .map_err(|e| format!("Tidak dapat menghubungi peladen: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("Peladen mengembalikan kode kesalahan: {}", res.status()));
        }

        let payload: serde_json::Value = res.json()
            .await
            .map_err(|e| format!("Respon dari peladen tidak valid (bukan JSON): {}", e))?;

        if !payload["success"].as_bool().unwrap_or(false) {
            return Err("Peladen menolak sambungan ping.".to_string());
        }

        let school_name = payload["school_name"].as_str().unwrap_or("Sekolah");
        return Ok(format!("Terhubung ke peladen web {}", school_name));
    }

    let url = format!("{}/api/v1/sync/pull?since=2026-08-01%2000:00:00", base_url);
    
    let res = client.get(&url)
        .header("X-API-Key", &api_key)
        .send()
        .await
        .map_err(|e| format!("Tidak dapat menghubungi peladen: {}", e))?;

    if res.status().as_u16() == 401 {
        return Err("API Key tidak valid (Ditolak oleh peladen).".to_string());
    }

    if !res.status().is_success() {
        return Err(format!("Peladen mengembalikan kode kesalahan: {}", res.status()));
    }

    let payload: serde_json::Value = res.json()
        .await
        .map_err(|e| format!("Respon dari peladen tidak valid (Gagal membaca JSON): {}", e))?;

    if !payload["success"].as_bool().unwrap_or(false) {
        return Err(payload["message"].as_str().unwrap_or("Peladen menolak permintaan.").to_string());
    }

    Ok("Koneksi berhasil dan API Key terverifikasi.".to_string())
}

#[tauri::command]
pub fn get_payment_receipt(transaction_code: String, state: State<'_, DbState>) -> Result<ReceiptData, String> {
    let conn = state.conn.lock().unwrap();
    
    // a. Query payment & student info
    let (payment_id, created_at, payment_method_name, total_amount, notes, student_nama, student_nis, student_nisn, student_kelas) = conn.query_row(
        "SELECT p.id, p.created_at, p.payment_method_name, p.total_amount, p.notes, s.nama, s.nis, s.nisn, c.name
         FROM payments p
         JOIN students s ON p.student_id = s.id
         LEFT JOIN classrooms c ON s.classroom_id = c.id
         WHERE p.transaction_code = ?;",
        [&transaction_code],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        }
    ).map_err(|e| format!("Transaksi tidak ditemukan: {}", e))?;
    
    // b. Query payment details
    let mut stmt = conn.prepare(
        "SELECT t.name, t.type, a.month, d.amount, y.name
         FROM payment_details d
         JOIN payment_assignments a ON d.payment_assignment_id = a.id
         JOIN payment_types t ON a.payment_type_id = t.id
         LEFT JOIN academic_years y ON a.academic_year_id = y.id
         WHERE d.payment_id = ?;"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([&payment_id], |row| {
        let name: String = row.get(0)?;
        let t_type: String = row.get(1)?;
        let month: Option<i32> = row.get(2)?;
        let amount: f64 = row.get(3)?;
        let year_name: Option<String> = row.get(4)?;
        
        let month_name = month.map(|m| match m {
            7 => "Juli".to_string(),
            8 => "Agustus".to_string(),
            9 => "September".to_string(),
            10 => "Oktober".to_string(),
            11 => "November".to_string(),
            12 => "Desember".to_string(),
            1 => "Januari".to_string(),
            2 => "Februari".to_string(),
            3 => "Maret".to_string(),
            4 => "April".to_string(),
            5 => "Mei".to_string(),
            6 => "Juni".to_string(),
            _ => format!("Bulan {}", m),
        });
        
        Ok(ReceiptItem {
            payment_type_name: name,
            payment_type_type: t_type,
            month,
            month_name,
            amount,
            academic_year_name: year_name,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut items = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            items.push(item);
        }
    }
    
    Ok(ReceiptData {
        transaction_code,
        created_at,
        payment_method_name,
        total_amount,
        notes,
        student_nama,
        student_nis,
        student_nisn,
        student_kelas,
        items,
    })
}

#[tauri::command]
pub fn get_all_payment_history(
    page: u32,
    search: String,
    state: State<'_, DbState>,
) -> Result<Vec<PaymentHistoryItem>, String> {
    let conn = state.conn.lock().unwrap();
    let offset = (page.max(1) - 1) * 30;
    let q = format!("%{}%", search);
    
    let mut stmt = conn.prepare(
        "SELECT p.id, p.transaction_code, s.nama, s.kelas, p.total_amount, p.payment_method_name, p.notes, p.created_at, p.sync_status
         FROM payments p
         JOIN students s ON p.student_id = s.id
         WHERE p.transaction_code LIKE ? OR s.nama LIKE ? OR s.kelas LIKE ?
         ORDER BY p.created_at DESC
         LIMIT 30 OFFSET ?;"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map(params![q, q, q, offset], |row| {
        Ok(PaymentHistoryItem {
            id: row.get(0)?,
            transaction_code: row.get(1)?,
            student_name: row.get(2)?,
            student_class: row.get(3).unwrap_or_default(),
            total_amount: row.get(4)?,
            payment_method_name: row.get(5)?,
            notes: row.get(6)?,
            created_at: row.get(7)?,
            sync_status: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            list.push(item);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn get_local_reports(state: State<'_, DbState>) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().unwrap();
    
    // 1. Total penerimaan per metode bayar
    let mut method_stmt = conn.prepare(
        "SELECT payment_method_name, SUM(total_amount), COUNT(*) 
         FROM payments 
         GROUP BY payment_method_name;"
    ).map_err(|e| e.to_string())?;
    let method_rows = method_stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let total: f64 = row.get(1)?;
        let count: i64 = row.get(2)?;
        Ok(serde_json::json!({
            "method": name,
            "total": total,
            "count": count
        }))
    }).map_err(|e| e.to_string())?;
    let mut methods = Vec::new();
    for r in method_rows {
        if let Ok(item) = r {
            methods.push(item);
        }
    }

    // 2. Penerimaan per Pos Pembayaran
    let mut pos_stmt = conn.prepare(
        "SELECT t.name, SUM(d.amount)
         FROM payment_details d
         JOIN payment_assignments a ON d.payment_assignment_id = a.id
         JOIN payment_types t ON a.payment_type_id = t.id
         GROUP BY t.name;"
    ).map_err(|e| e.to_string())?;
    let pos_rows = pos_stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let total: f64 = row.get(1)?;
        Ok(serde_json::json!({
            "pos": name,
            "total": total
        }))
    }).map_err(|e| e.to_string())?;
    let mut pos_breakdown = Vec::new();
    for r in pos_rows {
        if let Ok(item) = r {
            pos_breakdown.push(item);
        }
    }

    // 3. Ringkasan umum: total transaksi, total terbayar, total tunggakan tersisa
    let total_collected: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments;",
        [],
        |row| row.get(0)
    ).unwrap_or(0.0);

    let total_txs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payments;",
        [],
        |row| row.get(0)
    ).unwrap_or(0);

    let remaining_receivables: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount - paid_amount - relief_amount), 0.0) FROM payment_assignments;",
        [],
        |row| row.get(0)
    ).unwrap_or(0.0);

    Ok(serde_json::json!({
        "total_collected": total_collected,
        "total_txs": total_txs,
        "remaining_receivables": remaining_receivables,
        "methods": methods,
        "pos_breakdown": pos_breakdown
    }))
}

/// Download logo & aset gambar sekolah dari server, simpan sebagai base64 Data URL di settings lokal.
/// Dipanggil sekali saat onboarding selesai dan setiap setelah sync pull.
#[tauri::command]
pub async fn download_school_assets(state: State<'_, DbState>) -> Result<String, String> {
    // 1. Ambil api_url, api_key, dan setting gambar dari DB (kunci sesingkat mungkin)
    let (api_url, api_key, logo_left_url, logo_right_url, stamp_url) = {
        let conn = state.conn.lock().unwrap();
        let (url, key) = get_sync_settings(&conn);

        let logo_left: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'logo_left';", [],
            |row| row.get(0),
        ).unwrap_or_default();

        let logo_right: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'logo_right';", [],
            |row| row.get(0),
        ).unwrap_or_default();

        let stamp: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'stamp_image';", [],
            |row| row.get(0),
        ).unwrap_or_default();

        (url, key, logo_left, logo_right, stamp)
    };

    if api_url.is_empty() {
        return Err("API URL belum dikonfigurasi".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Gagal membuat HTTP client: {}", e))?;

    let base_url = api_url.trim_end_matches('/').to_string();

    let logo_left_b64  = fetch_image_as_base64(&client, &api_key, &base_url, &logo_left_url).await;
    let logo_right_b64 = fetch_image_as_base64(&client, &api_key, &base_url, &logo_right_url).await;
    let stamp_b64      = fetch_image_as_base64(&client, &api_key, &base_url, &stamp_url).await;

    // 2. Simpan kembali ke settings sebagai base64 Data URL
    let conn = state.conn.lock().unwrap();
    let mut updated = 0u32;

    if let Some(ref b64) = logo_left_b64 {
        conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'logo_left', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [b64],
        ).map_err(|e| format!("Gagal simpan logo_left: {}", e))?;
        updated += 1;
    }
    if let Some(ref b64) = logo_right_b64 {
        conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'logo_right', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [b64],
        ).map_err(|e| format!("Gagal simpan logo_right: {}", e))?;
        updated += 1;
    }
    if let Some(ref b64) = stamp_b64 {
        conn.execute(
            "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'stamp_image', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            [b64],
        ).map_err(|e| format!("Gagal simpan stamp_image: {}", e))?;
        updated += 1;
    }

    Ok(format!("{} aset gambar berhasil diunduh dan disimpan lokal.", updated))
}

/// Download satu URL gambar dan kembalikan sebagai base64 Data URL.
/// Jika gambar sudah berupa data URL atau URL kosong, dikembalikan apa adanya / None.
async fn fetch_image_as_base64(
    client: &reqwest::Client,
    api_key: &str,
    base_url: &str,
    url_value: &str,
) -> Option<String> {
    if url_value.is_empty() {
        return None;
    }
    // Sudah base64 data URL → skip download
    if url_value.starts_with("data:") {
        return Some(url_value.to_string());
    }
    // Resolve URL relatif
    let full_url = if url_value.starts_with("http://") || url_value.starts_with("https://") {
        url_value.to_string()
    } else {
        let sep = if url_value.starts_with('/') { "" } else { "/" };
        format!("{}{}{}", base_url, sep, url_value)
    };

    let res = client.get(&full_url)
        .header("X-API-Key", api_key)
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        println!("Gagal download asset {}: {}", full_url, res.status());
        return None;
    }

    let content_type = res.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_string();

    let bytes = res.bytes().await.ok()?;
    let b64 = base64_encode(&bytes);
    Some(format!("data:{};base64,{}", content_type, b64))
}

/// Simple base64 encoder
fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((combined >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((combined >> 12) & 0x3F) as usize] as char);
        result.push(if chunk.len() > 1 { CHARS[((combined >> 6) & 0x3F) as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[(combined & 0x3F) as usize] as char } else { '=' });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
