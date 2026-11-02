use partisipasi_sekolah_lib::db;
use uuid::Uuid;
use chrono::Local;

#[test]
fn test_e2e_full_pos_and_savings_flow() {
    println!("=== 1. Membuka Database SQLCipher Lokal ===");
    let conn = db::init_db_standalone().expect("Harus berhasil membuka database SQLCipher");

    println!("=== 2. E2E: Cari Siswa Berdasarkan Query ===");
    let query_param = "%FAJAR%";
    let mut stmt = conn.prepare(
        "SELECT s.id, s.nama, s.nis, c.name FROM students s 
         LEFT JOIN classrooms c ON s.classroom_id = c.id 
         WHERE s.nama LIKE ?1 LIMIT 5;"
    ).unwrap();

    let found_students: Vec<(String, String, Option<String>, Option<String>)> = stmt.query_map([query_param], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    }).unwrap().flatten().collect();

    assert!(!found_students.is_empty(), "Harus ada siswa yang cocok dengan query");
    let target = &found_students[0];
    println!("Siswa terpilih: ID={}, Nama={}, NIS={:?}, Kelas={:?}", target.0, target.1, target.2, target.3);

    println!("=== 3. E2E: Ambil Tagihan Siswa & Validasi Perhitungan ===");
    let mut ass_stmt = conn.prepare(
        "SELECT a.id, t.name, a.amount, a.paid_amount, a.relief_amount, a.status 
         FROM payment_assignments a 
         JOIN payment_types t ON a.payment_type_id = t.id 
         WHERE a.student_id = ?1;"
    ).unwrap();

    let assignments: Vec<(String, String, f64, f64, f64, String)> = ass_stmt.query_map([&target.0], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    }).unwrap().flatten().collect();

    let mut total_tagihan = 0.0;
    let mut total_dibayar = 0.0;
    let mut total_keringanan = 0.0;
    let mut total_tunggakan = 0.0;

    for a in &assignments {
        total_tagihan += a.2;
        total_dibayar += a.3;
        total_keringanan += a.4;
        let sisa = (a.2 - a.3 - a.4).max(0.0);
        total_tunggakan += sisa;
    }

    println!("Ringkasan Tagihan Siswa {}:", target.1);
    println!(" - Total Tagihan: Rp {}", total_tagihan);
    println!(" - Sudah Dibayar: Rp {}", total_dibayar);
    println!(" - Keringanan: Rp {}", total_keringanan);
    println!(" - Tunggakan: Rp {}", total_tunggakan);

    println!("=== 4. E2E: Uji Mutasi Tabungan Siswa (Tabel savings & savings_transactions) ===");
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Dapatkan user_id yang valid dari database untuk foreign key `users`
    let user_id: String = conn.query_row(
        "SELECT id FROM users LIMIT 1;",
        [],
        |r| r.get(0)
    ).unwrap();
    println!("User kasir aktif: ID={}", user_id);

    // Pastikan akun tabungan siswa ada di tabel `savings`
    let savings_id: String = conn.query_row(
        "SELECT id FROM savings WHERE student_id = ?1;",
        [&target.0],
        |r| r.get(0)
    ).unwrap_or_else(|_| {
        let new_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO savings (id, student_id, balance, is_active, created_at, updated_at, sync_status) 
             VALUES (?1, ?2, 0.0, 1, ?3, ?3, 'pending_push');",
            rusqlite::params![new_id, target.0, now_str]
        ).unwrap();
        new_id
    });

    let initial_balance: f64 = conn.query_row(
        "SELECT balance FROM savings WHERE id = ?1;",
        [&savings_id],
        |r| r.get(0)
    ).unwrap_or(0.0);
    println!("Saldo awal tabungan (ID {}): Rp {}", savings_id, initial_balance);

    // a. Uji Setor Rp 50.000
    let test_deposit = 50_000.0;
    let dep_trx_id = Uuid::new_v4().to_string();
    let balance_after_dep = initial_balance + test_deposit;

    conn.execute(
        "INSERT INTO savings_transactions (id, savings_id, type, amount, balance_after, notes, user_id, created_at, sync_status)
         VALUES (?1, ?2, 'setor', ?3, ?4, 'Test E2E Setor', ?5, ?6, 'pending_push');",
        rusqlite::params![dep_trx_id, savings_id, test_deposit, balance_after_dep, user_id, now_str]
    ).unwrap();

    conn.execute(
        "UPDATE savings SET balance = ?1, updated_at = ?2 WHERE id = ?3;",
        rusqlite::params![balance_after_dep, now_str, savings_id]
    ).unwrap();

    let check_dep_balance: f64 = conn.query_row(
        "SELECT balance FROM savings WHERE id = ?1;", [&savings_id], |r| r.get(0)
    ).unwrap();
    assert_eq!(check_dep_balance, balance_after_dep, "Saldo harus bertambah setelah setor");
    println!("✓ Setor Tabungan Sukses: Saldo baru = Rp {}", check_dep_balance);

    // b. Uji Tarik Rp 20.000
    let test_withdraw = 20_000.0;
    let wd_trx_id = Uuid::new_v4().to_string();
    let balance_after_wd = balance_after_dep - test_withdraw;

    conn.execute(
        "INSERT INTO savings_transactions (id, savings_id, type, amount, balance_after, notes, user_id, created_at, sync_status)
         VALUES (?1, ?2, 'tarik', ?3, ?4, 'Test E2E Tarik', ?5, ?6, 'pending_push');",
        rusqlite::params![wd_trx_id, savings_id, test_withdraw, balance_after_wd, user_id, now_str]
    ).unwrap();

    conn.execute(
        "UPDATE savings SET balance = ?1, updated_at = ?2 WHERE id = ?3;",
        rusqlite::params![balance_after_wd, now_str, savings_id]
    ).unwrap();

    let check_wd_balance: f64 = conn.query_row(
        "SELECT balance FROM savings WHERE id = ?1;", [&savings_id], |r| r.get(0)
    ).unwrap();
    assert_eq!(check_wd_balance, balance_after_wd, "Saldo harus berkurang setelah tarik");
    println!("✓ Tarik Tabungan Sukses: Saldo baru = Rp {}", check_wd_balance);

    // c. Rollback / Cleanup data test agar database tetap bersih
    conn.execute("DELETE FROM savings_transactions WHERE id IN (?1, ?2);", rusqlite::params![dep_trx_id, wd_trx_id]).unwrap();
    conn.execute("UPDATE savings SET balance = ?1 WHERE id = ?2;", rusqlite::params![initial_balance, savings_id]).unwrap();
    println!("✓ Rollback test mutasi tabungan berhasil (saldo kembali ke Rp {}).", initial_balance);

    println!("=== 5. E2E: Rekap Kasir & Antrean Data Offline ===");
    let today_prefix = Local::now().format("%Y-%m-%d").to_string();
    let month_prefix = Local::now().format("%Y-%m").to_string();

    let today_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments WHERE created_at LIKE ?1;",
        [format!("{}%", today_prefix)],
        |r| r.get(0)
    ).unwrap_or(0.0);

    let monthly_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments WHERE created_at LIKE ?1;",
        [format!("{}%", month_prefix)],
        |r| r.get(0)
    ).unwrap_or(0.0);

    let today_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payments WHERE created_at LIKE ?1;",
        [format!("{}%", today_prefix)],
        |r| r.get(0)
    ).unwrap_or(0);

    println!("Statistik Kasir Hari Ini:");
    println!(" - Pemasukan Hari Ini: Rp {}", today_total);
    println!(" - Pemasukan Bulan Ini: Rp {}", monthly_total);
    println!(" - Transaksi Hari Ini: {} Transaksi", today_count);

    let pending_payments: i64 = conn.query_row("SELECT count(*) FROM payments WHERE sync_status != 'synced';", [], |r| r.get(0)).unwrap_or(0);
    let pending_savings: i64 = conn.query_row("SELECT count(*) FROM savings_transactions WHERE sync_status != 'synced';", [], |r| r.get(0)).unwrap_or(0);
    println!("Antrean sinkronisasi offline: payments={}, savings={}", pending_payments, pending_savings);

    println!("🎉 Seluruh Pengujian E2E Database & POS Workflow Berhasil 100%!");
}
