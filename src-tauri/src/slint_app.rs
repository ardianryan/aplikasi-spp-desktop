use std::sync::{Arc, Mutex};
use std::rc::Rc;
use slint::{Model, VecModel};
use rusqlite::{params, Connection};
use chrono::Local;
use uuid::Uuid;

slint::include_modules!();

pub fn format_idr(amount: f64) -> String {
    let int_part = amount.round() as i64;
    let is_negative = int_part < 0;
    let abs_val = int_part.abs();
    let s = abs_val.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().rev().collect();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('.');
        }
        result.push(*ch);
    }
    let formatted: String = result.chars().rev().collect();
    if is_negative {
        format!("-Rp {}", formatted)
    } else {
        format!("Rp {}", formatted)
    }
}

pub struct ActiveCartItem {
    pub assignment_id: String,
    pub name: String,
    pub remaining: f64,
    pub pay_amount: f64,
}

pub struct SlintAppState {
    pub selected_student_id: Option<String>,
    pub cart: Vec<ActiveCartItem>,
    pub selected_method: i32, // 0: Tunai, 1: Transfer, 2: Tabungan
    pub active_year_id: Option<String>,
    pub user_token: Option<String>,
    pub user_nama: String,
    pub user_role: String,
    pub is_logged_in: bool,
    pub collapsed_years: std::collections::HashSet<String>,
    pub student_filter_status: i32, // 0: Siswa Aktif, 1: Non-Aktif
}

fn get_launch_agent_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/LaunchAgents/me.ppti.partisipasi-sekolah.plist"))
}

fn is_autostart_active() -> bool {
    if let Some(path) = get_launch_agent_path() {
        path.exists()
    } else {
        false
    }
}

fn set_autostart_active(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let plist_path = match get_launch_agent_path() {
        Some(p) => p,
        None => return Err("Direktori pengguna tidak ditemukan".into()),
    };

    if enabled {
        let exe_path = std::env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();
        if let Some(parent) = plist_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let plist_content = format!(
r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>me.ppti.partisipasi-sekolah</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>"#,
            exe_str
        );
        std::fs::write(&plist_path, plist_content)?;
    } else {
        if plist_path.exists() {
            let _ = std::fs::remove_file(&plist_path);
        }
    }
    Ok(())
}

fn check_updates_background(app_weak: slint::Weak<AppWindow>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new();
        if let Ok(rt) = rt {
            rt.block_on(async {
                let endpoint = "https://github.com/ardianryan/aplikasi-spp-desktop/releases/latest/download/latest.json";
                let client = match reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                match client.get(endpoint).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(val) = resp.json::<serde_json::Value>().await {
                            let remote_ver = val.get("version").and_then(|v| v.as_str()).unwrap_or("0.2.0");
                            let current_ver = "0.2.0";
                            let clean_remote = remote_ver.trim_start_matches('v');
                            let clean_current = current_ver.trim_start_matches('v');
                            let status_text = if clean_remote == clean_current {
                                format!("Aplikasi sudah menggunakan versi terbaru (v{}).", current_ver)
                            } else {
                                format!("Tersedia versi baru (v{}). Silakan unduh di GitHub Release.", clean_remote)
                            };
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(app) = app_weak.upgrade() {
                                    app.set_update_status_text(status_text.into());
                                }
                            });
                        }
                    }
                    _ => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                app.set_update_status_text("Versi resmi v0.2.0 (Arsitektur Native Offline).".into());
                            }
                        });
                    }
                }
            });
        }
    });
}

pub fn run_slint_native() -> Result<(), Box<dyn std::error::Error>> {
    println!("[INFO] Menjalankan Partisipasi Sekolah Desktop v0.2.0 (100% Native Rust & Slint UI)...");

    // Deteksi argumen baris perintah untuk deep-linking (partisipasi-sekolah://)
    let args: Vec<String> = std::env::args().collect();
    let maybe_deep_link = args.iter().find(|a| a.starts_with("partisipasi-sekolah://") || a.starts_with("partisipasi://")).cloned();

    // Pastikan single-instance: jika aplikasi sudah berjalan, alihkan fokus/deep-link lalu keluar
    #[cfg(unix)]
    {
        let socket_path = "/tmp/partisipasi_desktop_ipc.sock";
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
            use std::io::Write;
            let msg = if let Some(ref url) = maybe_deep_link {
                url.clone()
            } else {
                "ACTIVATE".to_string()
            };
            let _ = writeln!(stream, "{}", msg);
            let _ = stream.flush();
            println!("[INFO] Instance aplikasi sudah berjalan. Perintah '{}' dikirim ke instance aktif.", msg);
            return Ok(());
        }
    }

    // 1. Inisialisasi Database SQLite lokal
    let conn = crate::db::init_db_standalone()
        .map_err(|e| format!("Gagal inisialisasi SQLite database: {}", e))?;

    // 2. Jalankan Migrasi Database otomatis
    crate::db::run_migrations(&conn)
        .map_err(|e| format!("Gagal menjalankan migrasi database: {}", e))?;

    let conn_arc = Arc::new(Mutex::new(conn));

    // 3. Buat Window Utama Slint
    let app = AppWindow::new()?;
    let app_weak = app.as_weak();

    // Inisialisasi State lokal
    let state = Arc::new(Mutex::new(SlintAppState {
        selected_student_id: None,
        cart: Vec::new(),
        selected_method: 0,
        active_year_id: None,
        user_token: None,
        user_nama: "Kasir Loket 1".to_string(),
        user_role: "Petugas Kasir".to_string(),
        is_logged_in: false,
        collapsed_years: std::collections::HashSet::new(),
        student_filter_status: 0,
    }));

    // Close-to-Tray (Sembunyikan ke background saat tombol tutup ditekan agar autosync tetap aktif)
    app.window().on_close_requested(|| {
        println!("[INFO] Tombol close ditekan. Menyembunyikan jendela ke background agar antrean autosync tetap beroperasi.");
        slint::CloseRequestResponse::HideWindow
    });

    // Inisialisasi status autostart & pembaruan sistem di latar belakang
    app.set_autostart_enabled(is_autostart_active());
    check_updates_background(app.as_weak());

    // Cek apakah instalasi baru (fresh install) jika belum ada api_url yang terkonfigurasi
    {
        if let Ok(conn) = conn_arc.lock() {
            let api_url: String = conn.query_row(
                "SELECT value FROM settings WHERE key = 'api_url';",
                [],
                |r| r.get(0),
            ).unwrap_or_default();
            app.set_is_fresh_install(api_url.trim().is_empty());
        }
    }

    // Cek Konfigurasi Server & PIN Offline
    {
        if let Ok(conn) = conn_arc.lock() {
            update_branding_and_settings(&app, &conn);

            let api_url: String = conn.query_row("SELECT value FROM settings WHERE key = 'api_url';", [], |r| r.get(0))
                .unwrap_or_else(|_| "http://sumbangan.smage.my.id".to_string());
            app.set_api_endpoint_url(api_url.into());

            let has_pin = crate::commands::has_offline_pin_conn(&conn).unwrap_or(false);
            if !has_pin {
                let _ = crate::commands::set_offline_pin_conn(&conn, "123456");
            }
            app.set_has_offline_pin(true);
        }
    }

    // Lakukan pemeriksaan status koneksi server secara cepat saat startup
    {
        let app_weak_init = app.as_weak();
        let conn_arc_init = Arc::clone(&conn_arc);
        std::thread::spawn(move || {
            let api_url = if let Ok(conn) = conn_arc_init.lock() {
                conn.query_row("SELECT value FROM settings WHERE key = 'api_url';", [], |r| r.get::<_, String>(0))
                    .unwrap_or_else(|_| "http://sumbangan.smage.my.id".to_string())
            } else {
                "http://sumbangan.smage.my.id".to_string()
            };
            let ping_url = format!("{}/api/v1/ping", api_url.trim_end_matches('/'));
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(async {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(4))
                        .build();
                    if let Ok(c) = client {
                        if let Ok(resp) = c.get(&ping_url).send().await {
                            let online = resp.status().is_success();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(app) = app_weak_init.upgrade() {
                                    app.set_is_online(online);
                                }
                            });
                        }
                    }
                });
            }
        });
    }

    // Cek Tahun Ajaran Aktif dari DB
    {
        if let Ok(conn) = conn_arc.lock() {
            if let Ok(mut stmt) = conn.prepare("SELECT id, name FROM academic_years WHERE is_active = 1 LIMIT 1;") {
                if let Ok(mut rows) = stmt.query([]) {
                    if let Ok(Some(row)) = rows.next() {
                        let id: String = row.get(0).unwrap_or_default();
                        let name: String = row.get(1).unwrap_or_else(|_| "2024/2025".into());
                        state.lock().unwrap().active_year_id = Some(id);
                        app.set_active_academic_year(name.into());
                    }
                }
            }

            // Cek Sesi Pengguna Aktif dari Database
            let (has_session, user_name, user_role) = {
                let res = conn.query_row(
                    "SELECT ut.token, u.nama, u.role 
                     FROM user_tokens ut 
                     LEFT JOIN users u ON ut.user_id = u.id 
                     ORDER BY ut.created_at DESC 
                     LIMIT 1;",
                    [],
                    |row| Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "Kasir Loket 1".to_string()),
                        row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "Petugas Kasir".to_string()),
                    ))
                );
                match res {
                    Ok((tok, nama, role)) => {
                        let mut st = state.lock().unwrap();
                        st.user_token = Some(tok);
                        st.user_nama = nama.clone();
                        st.user_role = role.clone();
                        st.is_logged_in = true;
                        (true, nama, role)
                    }
                    Err(_) => (false, "Kasir Loket 1".to_string(), "Petugas Kasir".to_string())
                }
            };
            app.set_is_logged_in(has_session);
            app.set_user_name(user_name.into());
            app.set_user_role(user_role.into());
        }
    }

    // Muat Riwayat Transaksi Hari Ini saat aplikasi dibuka
    refresh_today_payments_ui(&app, &conn_arc);
    refresh_sync_status_ui(&app, &conn_arc);

    // Setup Local Socket IPC Server untuk menerima deep-link saat aplikasi berjalan
    #[cfg(unix)]
    {
        let socket_path = "/tmp/partisipasi_desktop_ipc.sock";
        let _ = std::fs::remove_file(socket_path);
        if let Ok(listener) = std::os::unix::net::UnixListener::bind(socket_path) {
            let app_weak_ipc = app.as_weak();
            let conn_arc_ipc = Arc::clone(&conn_arc);
            let state_ipc = Arc::clone(&state);

            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if let Ok(mut s) = stream {
                        use std::io::{BufRead, BufReader};
                        let mut reader = BufReader::new(&mut s);
                        let mut line = String::new();
                        if reader.read_line(&mut line).is_ok() {
                            let trimmed = line.trim().to_string();
                            if !trimmed.is_empty() {
                                let app_weak = app_weak_ipc.clone();
                                let conn_arc = Arc::clone(&conn_arc_ipc);
                                let state = Arc::clone(&state_ipc);
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(app) = app_weak.upgrade() {
                                        let _ = app.window().show();
                                        if trimmed != "ACTIVATE" {
                                            process_deep_link_url(&trimmed, &app, &conn_arc, &state);
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            });
        }
    }

    // Jika biner diluncurkan langsung membawa URL deep-link
    if let Some(ref url) = maybe_deep_link {
        process_deep_link_url(url, &app, &conn_arc, &state);
    }

    // ==========================================
    // EVENT LISTENERS & CALLBACKS
    // ==========================================

    // A. Tab Navigation Changed
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_tab_changed(move |tab_idx| {
            if let Some(app) = app_weak.upgrade() {
                if tab_idx == 2 {
                    refresh_history_tab_ui(&app, &conn_arc, "");
                } else if tab_idx == 3 {
                    refresh_reports_tab_ui(&app, &conn_arc);
                } else if tab_idx == 4 {
                    refresh_sync_status_ui(&app, &conn_arc);
                }
            }
        });
    }

    // B. Search Student Query Changed & Filter Status
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_search_changed(move |query| {
            if let Some(app) = app_weak.upgrade() {
                perform_student_search(&app, &conn_arc, &state, &query);
            }
        });
    }

    // B2. Student Filter Status Changed (0 = Siswa Aktif, 1 = Non-Aktif / Alumni)
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_student_filter_changed(move |filter_status| {
            if let Some(app) = app_weak.upgrade() {
                {
                    let mut st = state.lock().unwrap();
                    st.student_filter_status = filter_status;
                }
                let query = app.get_search_query().to_string();
                perform_student_search(&app, &conn_arc, &state, &query);
            }
        });
    }

    // C. Student Selected from Search
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_student_selected(move |student_id| {
            if let Some(app) = app_weak.upgrade() {
                app.set_show_search_results(false);
                app.set_search_query("".into());
                state.lock().unwrap().selected_student_id = Some(student_id.to_string());
                state.lock().unwrap().cart.clear();

                load_student_details_ui(&app, &conn_arc, &state, &student_id, true);
            }
        });
    }

    // C2. Clear Selected Student (Kembali ke Welcome Banner)
    {
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_clear_student(move || {
            if let Some(app) = app_weak.upgrade() {
                state.lock().unwrap().selected_student_id = None;
                state.lock().unwrap().cart.clear();
                app.set_current_student(StudentData {
                    id: "".into(),
                    nis: "-".into(),
                    nisn: "-".into(),
                    nama: "".into(),
                    kelas: "-".into(),
                    savings_balance: "Rp 0".into(),
                });
                app.set_assignments(Rc::new(VecModel::default()).into());
                app.set_cart_items(Rc::new(VecModel::default()).into());
                app.set_cart_total("Rp 0".into());
                app.set_bill_summary(BillSummaryData {
                    total_tagihan: "Rp 0".into(),
                    total_dibayar: "Rp 0".into(),
                    total_keringanan: "Rp 0".into(),
                    total_tunggakan: "Rp 0".into(),
                });
            }
        });
    }

    // D. Toggle Assignment Checkbox (Add to Cart / Remove)
    {
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_toggle_assignment(move |assignment_id| {
            if let Some(app) = app_weak.upgrade() {
                let mut state_lock = state.lock().unwrap();
                let aid_str = assignment_id.to_string();

                // Cari di cart, jika ada hapus
                let pos = state_lock.cart.iter().position(|c| c.assignment_id == aid_str);
                if let Some(idx) = pos {
                    state_lock.cart.remove(idx);
                } else {
                    // Cari di assignment list untuk ditambahkan
                    let assignments_model = app.get_assignments();
                    for i in 0..assignments_model.row_count() {
                        if let Some(item) = assignments_model.row_data(i) {
                            if item.id == aid_str && !item.is_group_header && item.remaining_raw > 0 {
                                state_lock.cart.push(ActiveCartItem {
                                    assignment_id: aid_str.clone(),
                                    name: format!("{} {}", item.payment_type_name, item.month_name).trim().to_string(),
                                    remaining: item.remaining_raw as f64,
                                    pay_amount: item.remaining_raw as f64,
                                });
                                break;
                            }
                        }
                    }
                }

                // Update UI assignments selection state & Cart list
                update_cart_and_selection_ui(&app, &state_lock);
            }
        });
    }

    // D2. Toggle Academic Year Group Accordion Collapse/Expand
    {
        let state = Arc::clone(&state);
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_toggle_year_collapse(move |year_name_slint| {
            if let Some(app) = app_weak.upgrade() {
                let year_str = year_name_slint.to_string();
                let student_id_opt = {
                    let mut st = state.lock().unwrap();
                    if st.collapsed_years.contains(&year_str) {
                        st.collapsed_years.remove(&year_str);
                    } else {
                        st.collapsed_years.insert(year_str.clone());
                    }
                    st.selected_student_id.clone()
                };

                if let Some(sid) = student_id_opt {
                    load_student_details_ui(&app, &conn_arc, &state, &sid, false);
                }
            }
        });
    }

    // E. Remove Cart Item
    {
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_remove_cart(move |assignment_id| {
            if let Some(app) = app_weak.upgrade() {
                let mut state_lock = state.lock().unwrap();
                let aid_str = assignment_id.to_string();
                state_lock.cart.retain(|c| c.assignment_id != aid_str);
                update_cart_and_selection_ui(&app, &state_lock);
            }
        });
    }

    // F. Set Payment Method (0: Tunai, 1: Transfer, 2: Tabungan)
    {
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_set_payment_method(move |method_idx| {
            state.lock().unwrap().selected_method = method_idx;
            if let Some(app) = app_weak.upgrade() {
                app.set_selected_payment_method(method_idx);
            }
        });
    }

    // G. Process Payment Execution
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_process_payment(move || {
            if let Some(app) = app_weak.upgrade() {
                let state_lock = state.lock().unwrap();
                let student_id = match &state_lock.selected_student_id {
                    Some(id) => id.clone(),
                    None => return,
                };

                if state_lock.cart.is_empty() {
                    return;
                }

                app.set_is_processing_payment(true);

                // Hitung total bayar
                let total_amount: f64 = state_lock.cart.iter().map(|c| c.pay_amount).sum();
                let method_name = match state_lock.selected_method {
                    1 => "Transfer",
                    2 => "Tabungan",
                    _ => "Tunai",
                };

                let method_id = match state_lock.selected_method {
                    1 => "transfer",
                    2 => "tabungan",
                    _ => "cash",
                };

                let academic_year_id = state_lock.active_year_id.clone();
                let cashier_nama = state_lock.user_nama.clone();

                // Generate kode transaksi: TRX-YYYYMMDD-HHMMSS-RAND
                let now = Local::now();
                let tx_code = format!("TRX-{}-{}", now.format("%Y%m%d%H%M%S"), &Uuid::new_v4().to_string()[..4].to_uppercase());
                let tx_id = Uuid::new_v4().to_string();
                let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

                drop(state_lock);

                // Eksekusi Simpan ke SQLite
                let mut success = false;
                if let Ok(mut conn) = conn_arc.lock() {
                    let tx = conn.transaction();
                    if let Ok(tx) = tx {
                        let user_id = get_active_user_id(&tx);

                        // Insert ke payments sesuai schema SQLite & sync engine
                        let res_p = tx.execute(
                            "INSERT INTO payments (id, transaction_code, student_id, user_id, total_amount, payment_method_id, payment_method_name, notes, academic_year_id, created_at, sync_status)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '', ?8, ?9, 'pending_push');",
                            params![tx_id, tx_code, student_id, user_id, total_amount, method_id, method_name, academic_year_id, now_str]
                        );

                        if res_p.is_ok() {
                            let state_lock = state.lock().unwrap();
                            for item in &state_lock.cart {
                                let det_id = Uuid::new_v4().to_string();
                                let _ = tx.execute(
                                    "INSERT INTO payment_details (id, payment_id, payment_assignment_id, amount, notes, created_at, sync_status)
                                     VALUES (?1, ?2, ?3, ?4, '', ?5, 'pending_push');",
                                    params![det_id, tx_id, item.assignment_id, item.pay_amount, now_str]
                                );

                                // Update payment_assignments paid_amount & status
                                let _ = tx.execute(
                                    "UPDATE payment_assignments 
                                     SET paid_amount = paid_amount + ?1,
                                         status = CASE 
                                             WHEN (paid_amount + ?1 + relief_amount) >= amount THEN 'lunas'
                                             ELSE 'sebagian'
                                         END,
                                         sync_status = 'pending_push'
                                     WHERE id = ?2;",
                                    params![item.pay_amount, item.assignment_id]
                                );
                            }

                            // Jika debet dari Tabungan Siswa
                            if method_id == "tabungan" {
                                let savings_id: String = tx.query_row(
                                    "SELECT id FROM savings WHERE student_id = ?1;",
                                    [&student_id],
                                    |r| r.get(0)
                                ).unwrap_or_else(|_| {
                                    let new_id = Uuid::new_v4().to_string();
                                    let _ = tx.execute(
                                        "INSERT INTO savings (id, student_id, balance, is_active, created_at, updated_at, sync_status) 
                                         VALUES (?1, ?2, 0.0, 1, ?3, ?3, 'pending_push');",
                                        params![new_id, student_id, now_str]
                                    );
                                    new_id
                                });

                                let cur_bal: f64 = tx.query_row(
                                    "SELECT balance FROM savings WHERE id = ?1;",
                                    [&savings_id],
                                    |r| r.get(0)
                                ).unwrap_or(0.0);

                                let new_bal = cur_bal - total_amount;
                                let _ = tx.execute(
                                    "INSERT INTO savings_transactions (id, savings_id, type, amount, balance_after, notes, user_id, created_at, sync_status)
                                     VALUES (?1, ?2, 'tarik', ?3, ?4, ?5, ?6, ?7, 'pending_push');",
                                    params![Uuid::new_v4().to_string(), savings_id, total_amount, new_bal, format!("Pembayaran {}", tx_code), user_id, now_str]
                                );
                                let _ = tx.execute(
                                    "UPDATE savings SET balance = ?1, updated_at = ?2, sync_status = 'pending_push' WHERE id = ?3;",
                                    params![new_bal, now_str, savings_id]
                                );
                            }

                            if tx.commit().is_ok() {
                                success = true;
                            }
                        }
                    }
                }

                app.set_is_processing_payment(false);

                if success {
                    state.lock().unwrap().cart.clear();
                    // Refresh data siswa & ringkasan
                    load_student_details_ui(&app, &conn_arc, &state, &student_id, false);
                    refresh_today_payments_ui(&app, &conn_arc);
                    refresh_sync_status_ui(&app, &conn_arc);

                    // Buka modal kwitansi otomatis di dalam aplikasi (Tanpa keluar ke peramban)
                    if let Ok(conn) = conn_arc.lock() {
                        let _ = populate_receipt_modal(&app, &conn, &tx_code, &cashier_nama);
                    }
                    app.set_sync_toast_message(format!("Pembayaran {} berhasil!", tx_code).into());
                    app.set_sync_toast_is_error(false);
                }
            }
        });
    }

    // H. Trigger Cloud Sync
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_trigger_sync(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_is_syncing(true);
                app.set_sync_toast_message("Sedang menghubungkan ke server cloud...".into());
                app.set_sync_toast_is_error(false);
                app.set_sync_message("Sedang menghubungkan ke server cloud...".into());

                let app_weak2 = app_weak.clone();
                let conn_arc2 = Arc::clone(&conn_arc);

                // Spawn task di background thread
                std::thread::spawn(move || {
                    let db_state = crate::db::DbState {
                        conn: Mutex::new(crate::db::init_db_standalone().unwrap()),
                    };

                    let rt = tokio::runtime::Runtime::new();
                    let result_msg = if let Ok(rt) = rt {
                        rt.block_on(async {
                            crate::commands::perform_sync(&db_state).await
                        })
                    } else {
                        Err("Gagal membuat runtime async".into())
                    };

                    let app_weak3 = app_weak2.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak2.upgrade() {
                            app.set_is_syncing(false);
                            match &result_msg {
                                Ok(msg) => {
                                    app.set_sync_message(msg.clone().into());
                                    app.set_sync_toast_message("Sinkronisasi Berhasil! Data lokal telah diperbarui.".into());
                                    app.set_sync_toast_is_error(false);
                                    app.set_is_online(true);
                                    println!("[INFO] Sinkronisasi Berhasil: {}", msg);
                                }
                                Err(err) => {
                                    app.set_sync_message(format!("Sync offline: {}", err).into());
                                    app.set_sync_toast_message(format!("Gagal Sinkron: {}", err).into());
                                    app.set_sync_toast_is_error(true);
                                    println!("[WARN] Sinkronisasi Gagal: {}", err);
                                }
                            }
                            if let Ok(conn) = conn_arc2.lock() {
                                update_branding_and_settings(&app, &conn);
                            }
                            refresh_sync_status_ui(&app, &conn_arc2);
                            refresh_today_payments_ui(&app, &conn_arc2);
                            refresh_history_tab_ui(&app, &conn_arc2, "");

                            // Auto-hide toast notification after 5 seconds
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_secs(5));
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(app) = app_weak3.upgrade() {
                                        app.set_sync_toast_message("".into());
                                    }
                                });
                            });
                        }
                    });
                });
            }
        });
    }

    // I. Savings Deposit & Withdraw
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_deposit_savings(move |amt_str, note_str| {
            if let Some(app) = app_weak.upgrade() {
                let parsed_amt: f64 = amt_str.trim().parse().unwrap_or(0.0);
                if parsed_amt <= 0.0 {
                    return;
                }
                let sid = state.lock().unwrap().selected_student_id.clone();
                if let Some(student_id) = sid {
                    if let Ok(conn) = conn_arc.lock() {
                        let _ = execute_savings_transaction(&conn, &student_id, "setor", parsed_amt, note_str.trim());
                    }
                    load_student_details_ui(&app, &conn_arc, &state, &student_id, false);
                }
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_withdraw_savings(move |amt_str, note_str| {
            if let Some(app) = app_weak.upgrade() {
                let parsed_amt: f64 = amt_str.trim().parse().unwrap_or(0.0);
                if parsed_amt <= 0.0 {
                    return;
                }
                let sid = state.lock().unwrap().selected_student_id.clone();
                if let Some(student_id) = sid {
                    if let Ok(conn) = conn_arc.lock() {
                        let _ = execute_savings_transaction(&conn, &student_id, "tarik", parsed_amt, note_str.trim());
                    }
                    load_student_details_ui(&app, &conn_arc, &state, &student_id, false);
                }
            }
        });
    }

    // J. Refresh History & Reports
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_refresh_history(move || {
            if let Some(app) = app_weak.upgrade() {
                refresh_history_tab_ui(&app, &conn_arc, "");
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_history_search_changed(move |q_slint| {
            if let Some(app) = app_weak.upgrade() {
                refresh_history_tab_ui(&app, &conn_arc, q_slint.as_str());
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_print_receipt(move |trx_code_slint| {
            let trx_code = trx_code_slint.to_string();
            if trx_code.is_empty() {
                return;
            }
            if let Some(app) = app_weak.upgrade() {
                if let Ok(conn) = conn_arc.lock() {
                    let cashier_name = state.lock().unwrap().user_nama.clone();
                    match populate_receipt_modal(&app, &conn, &trx_code, &cashier_name) {
                        Ok(_) => {
                            app.set_sync_toast_message("Kwitansi siap ditampilkan.".into());
                            app.set_sync_toast_is_error(false);
                        }
                        Err(e) => {
                            app.set_sync_toast_message(format!("Gagal memuat kwitansi: {}", e).into());
                            app.set_sync_toast_is_error(true);
                        }
                    }
                }
            }
        });
    }

    {
        let app_weak = app_weak.clone();
        app.on_close_receipt_modal(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_show_receipt_modal(false);
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_execute_direct_print(move |trx_code_slint| {
            let trx_code = trx_code_slint.to_string();
            if trx_code.is_empty() {
                return;
            }
            if let Some(app) = app_weak.upgrade() {
                if let Ok(conn) = conn_arc.lock() {
                    let cashier_name = state.lock().unwrap().user_nama.clone();
                    match send_receipt_to_printer(&conn, &trx_code, &cashier_name) {
                        Ok(_) => {
                            app.set_sync_toast_message("Kwitansi berhasil dikirim ke printer kasir.".into());
                            app.set_sync_toast_is_error(false);
                        }
                        Err(e) => {
                            app.set_sync_toast_message(format!("Info Cetak: {}", e).into());
                            app.set_sync_toast_is_error(true);
                        }
                    }
                }
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_refresh_reports(move || {
            if let Some(app) = app_weak.upgrade() {
                refresh_reports_tab_ui(&app, &conn_arc);
            }
        });
    }

    // K. Auth Handlers (Logout, SSO Login, PIN Login, Code Login)
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_logout(move || {
            if let Ok(conn) = conn_arc.lock() {
                let _ = crate::commands::logout_conn(&conn);
            }
            {
                let mut st = state.lock().unwrap();
                st.user_token = None;
                st.is_logged_in = false;
                st.selected_student_id = None;
                st.cart.clear();
            }
            if let Some(app) = app_weak.upgrade() {
                app.set_is_logged_in(false);
                app.set_login_error("".into());
                app.set_search_query("".into());
                app.set_current_student(StudentData {
                    id: "".into(),
                    nis: "-".into(),
                    nisn: "-".into(),
                    nama: "".into(),
                    kelas: "-".into(),
                    savings_balance: "Rp 0".into(),
                });
                app.set_assignments(Rc::new(VecModel::default()).into());
                app.set_cart_items(Rc::new(VecModel::default()).into());
                app.set_cart_total("Rp 0".into());
            }
            println!("[INFO] Kasir berhasil logout / keluar sesi.");
        });
    }

    // L. Save Server Host Configuration (from Login View before login)
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_save_server_url(move |raw_url_slint| {
            let mut raw_url = raw_url_slint.to_string().trim().to_string();
            if raw_url.is_empty() {
                if let Some(app) = app_weak.upgrade() {
                    app.set_login_error("Alamat peladen server tidak boleh kosong.".into());
                }
                return;
            }
            if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
                raw_url = format!("http://{}", raw_url);
            }
            let clean_url = raw_url.trim_end_matches('/').to_string();

            if let Ok(conn) = conn_arc.lock() {
                let _ = conn.execute(
                    "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_url', ?)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
                    [&clean_url],
                );
            }

            if let Some(app) = app_weak.upgrade() {
                app.set_api_endpoint_url(clean_url.clone().into());
                app.set_sync_base_url(clean_url.clone().into());
                app.set_login_error("".into());
            }
            println!("[INFO] Alamat endpoint peladen berhasil disimpan: {}", clean_url);
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        app.on_sso_login(move || {
            let api_url: String = {
                if let Ok(conn) = conn_arc.lock() {
                    conn.query_row("SELECT value FROM settings WHERE key = 'api_url';", [], |r| r.get(0))
                        .unwrap_or_else(|_| "https://sekolah.id".to_string())
                } else {
                    "https://sekolah.id".to_string()
                }
            };
            let clean_url = api_url.trim_end_matches('/');
            let login_url = format!("{}/auth/desktop-login", clean_url);
            println!("[INFO] Membuka SSO peramban: {}", login_url);
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(&login_url).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd").args(["/C", "start", &login_url]).spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(&login_url).spawn();
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_pin_login(move |pin_slint| {
            let pin_str = pin_slint.to_string();
            let clean_pin = pin_str.trim();
            if clean_pin.len() != 6 {
                if let Some(app) = app_weak.upgrade() {
                    app.set_login_error("PIN Kasir harus berupa 6 digit angka.".into());
                }
                return;
            }

            let result = {
                if let Ok(conn) = conn_arc.lock() {
                    crate::commands::verify_offline_pin_conn(&conn, clean_pin)
                } else {
                    Err("Gagal membuka database".into())
                }
            };

            match result {
                Ok(Some(sess)) => {
                    {
                        let mut st = state.lock().unwrap();
                        st.user_token = Some(sess.token);
                        st.user_nama = sess.nama.clone();
                        st.user_role = sess.role.clone();
                        st.is_logged_in = true;
                    }
                    if let Some(app) = app_weak.upgrade() {
                        app.set_user_name(sess.nama.into());
                        app.set_user_role(sess.role.into());
                        app.set_login_error("".into());
                        app.set_is_logged_in(true);
                        if let Ok(conn) = conn_arc.lock() {
                            update_branding_and_settings(&app, &conn);
                        }
                        refresh_today_payments_ui(&app, &conn_arc);
                        refresh_sync_status_ui(&app, &conn_arc);
                    }
                    println!("[INFO] Kasir berhasil masuk menggunakan PIN offline.");
                }
                Ok(None) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_login_error("PIN Kasir tidak cocok. Silakan coba lagi.".into());
                    }
                }
                Err(e) => {
                    if let Some(app) = app_weak.upgrade() {
                        app.set_login_error(format!("Gagal masuk: {}", e).into());
                    }
                }
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_code_login(move |code_slint| {
            let code_str = code_slint.to_string();
            let clean_code = code_str.trim().replace(" ", "").replace("-", "");
            if clean_code.len() < 6 {
                if let Some(app) = app_weak.upgrade() {
                    app.set_login_error("Kode otorisasi harus terdiri atas 6 digit angka.".into());
                }
                return;
            }

            let conn_arc = Arc::clone(&conn_arc);
            let state = Arc::clone(&state);
            let app_weak = app_weak.clone();

            std::thread::spawn(move || {
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                app.set_login_error(format!("Gagal runtime async: {}", e).into());
                            }
                        }).ok();
                        return;
                    }
                };

                let res = runtime.block_on(async {
                    let (api_url, api_key) = {
                        let conn = conn_arc.lock().map_err(|e| e.to_string())?;
                        crate::sync::get_sync_settings(&conn)
                    };
                    if api_url.is_empty() {
                        return Err("Alamat peladen web sekolah belum dikonfigurasi. Silakan periksa pengaturan sambungan.".to_string());
                    }
                    let exchange = crate::sync::exchange_code_with_server(&api_url, &api_key, &clean_code).await?;
                    let conn = conn_arc.lock().map_err(|e| e.to_string())?;
                    crate::commands::save_exchanged_session(&conn, exchange)
                });

                slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak.upgrade() {
                        match res {
                            Ok(sess) => {
                                {
                                    let mut st = state.lock().unwrap();
                                    st.user_token = Some(sess.token);
                                    st.user_nama = sess.nama.clone();
                                    st.user_role = sess.role.clone();
                                    st.is_logged_in = true;
                                }
                                app.set_user_name(sess.nama.into());
                                app.set_user_role(sess.role.into());
                                app.set_login_error("".into());
                                app.set_is_logged_in(true);
                                if let Ok(conn) = conn_arc.lock() {
                                    update_branding_and_settings(&app, &conn);
                                }
                                refresh_today_payments_ui(&app, &conn_arc);
                                refresh_sync_status_ui(&app, &conn_arc);
                                println!("[INFO] Kasir berhasil masuk dengan kode otorisasi.");
                            }
                            Err(e) => {
                                app.set_login_error(format!("Gagal masuk dengan kode: {}", e).into());
                            }
                        }
                    }
                }).ok();
            });
        });
    }

    // Save Sync Settings from Sync Tab (Zero-Config token managed automatically)
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_save_sync_settings(move |url_slint| {
            let mut raw_url = url_slint.to_string().trim().to_string();

            if raw_url.is_empty() {
                if let Some(app) = app_weak.upgrade() {
                    app.set_sync_toast_message("Alamat URL server tidak boleh kosong.".into());
                    app.set_sync_toast_is_error(true);
                }
                return;
            }
            if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
                raw_url = format!("http://{}", raw_url);
            }
            let clean_url = raw_url.trim_end_matches('/').to_string();

            if let Ok(conn) = conn_arc.lock() {
                let _ = conn.execute(
                    "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_url', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
                    [&clean_url],
                );
            }

            if let Some(app) = app_weak.upgrade() {
                app.set_sync_base_url(clean_url.clone().into());
                app.set_api_endpoint_url(clean_url.into());
                app.set_sync_toast_message("Pengaturan sambungan peladen berhasil disimpan.".into());
                app.set_sync_toast_is_error(false);
                refresh_sync_status_ui(&app, &conn_arc);
            }
        });
    }

    // Test Sync Connection from Sync Tab
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_test_sync_connection(move |url_slint| {
            let mut raw_url = url_slint.to_string().trim().to_string();

            if raw_url.is_empty() {
                if let Some(app) = app_weak.upgrade() {
                    app.set_test_connection_result("URL server tidak boleh kosong.".into());
                    app.set_sync_toast_message("URL server tidak boleh kosong.".into());
                    app.set_sync_toast_is_error(true);
                }
                return;
            }
            if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
                raw_url = format!("http://{}", raw_url);
            }
            let clean_url = raw_url.trim_end_matches('/').to_string();

            if let Some(app) = app_weak.upgrade() {
                app.set_is_testing_connection(true);
                app.set_test_connection_result("Menguji koneksi ke server...".into());
            }

            let app_weak2 = app_weak.clone();
            let token_opt = {
                if let Ok(conn) = conn_arc.lock() {
                    conn.query_row(
                        "SELECT value FROM settings WHERE key = 'api_key' UNION SELECT value FROM settings WHERE key = 'api_token' UNION SELECT token FROM user_tokens ORDER BY expires_at DESC LIMIT 1;",
                        [],
                        |r| r.get::<_, String>(0)
                    ).ok()
                } else {
                    None
                }
            };

            std::thread::spawn(move || {
                let ping_url = format!("{}/api/v1/ping", clean_url);
                let rt = tokio::runtime::Runtime::new();
                let res = if let Ok(rt) = rt {
                    rt.block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(6))
                            .build()
                            .map_err(|e| e.to_string())?;

                        let mut req = client.get(&ping_url);
                        if let Some(ref tok) = token_opt {
                            if !tok.is_empty() {
                                req = req.header("X-API-Key", tok)
                                         .header("Authorization", format!("Bearer {}", tok));
                            }
                        }
                        let resp = req.send().await.map_err(|e| format!("Tidak dapat menghubungi server: {}", e))?;
                        let status = resp.status();
                        if status.is_success() {
                            let payload: serde_json::Value = resp.json().await.unwrap_or_default();
                            let school = payload["school_name"].as_str().unwrap_or("Server Partisipasi");
                            Ok(format!("Terhubung ke peladen web {}", school))
                        } else if status.as_u16() == 401 {
                            Ok("Terhubung (Perlu otorisasi API key)".to_string())
                        } else {
                            Err(format!("Server merespons kode {}", status.as_u16()))
                        }
                    })
                } else {
                    Err("Gagal membuat runtime async".into())
                };

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak2.upgrade() {
                        app.set_is_testing_connection(false);
                        match res {
                            Ok(msg) => {
                                app.set_test_connection_result(format!("Koneksi berhasil! {}", msg).into());
                                app.set_sync_toast_message("Server terhubung dengan baik.".into());
                                app.set_sync_toast_is_error(false);
                                app.set_is_online(true);
                            }
                            Err(e) => {
                                app.set_test_connection_result(format!("Koneksi gagal: {}", e).into());
                                app.set_sync_toast_message(format!("Koneksi gagal: {}", e).into());
                                app.set_sync_toast_is_error(true);
                                app.set_is_online(false);
                            }
                        }
                    }
                });
            });
        });
    }

    // Toggle Autostart / Jalankan Otomatis saat Komputer Menyala
    {
        let app_weak = app_weak.clone();
        app.on_toggle_autostart_clicked(move || {
            let current = is_autostart_active();
            let new_state = !current;
            if let Err(e) = set_autostart_active(new_state) {
                eprintln!("[ERROR] Gagal mengubah status autostart: {}", e);
            }
            let active_now = is_autostart_active();
            if let Some(app) = app_weak.upgrade() {
                app.set_autostart_enabled(active_now);
                let msg = if active_now {
                    "Aplikasi akan otomatis berjalan saat komputer dinyalakan."
                } else {
                    "Jalankan otomatis saat startup dinonaktifkan."
                };
                app.set_sync_toast_message(msg.into());
                app.set_sync_toast_is_error(false);
            }
        });
    }

    // Setup Wizard Onboarding: Uji Sambungan Peladen
    {
        let app_weak = app_weak.clone();
        app.on_test_wizard_connection(move |url_slint| {
            let mut raw_url = url_slint.to_string().trim().to_string();
            if raw_url.is_empty() {
                if let Some(app) = app_weak.upgrade() {
                    app.set_wizard_test_message("URL server tidak boleh kosong.".into());
                    app.set_wizard_test_success(false);
                }
                return;
            }
            if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
                raw_url = format!("http://{}", raw_url);
            }
            let clean_url = raw_url.trim_end_matches('/').to_string();

            if let Some(app) = app_weak.upgrade() {
                app.set_is_testing_wizard(true);
                app.set_wizard_test_message("Menguji koneksi ke peladen...".into());
            }

            let app_weak2 = app_weak.clone();
            std::thread::spawn(move || {
                let ping_url = format!("{}/api/v1/sync/status", clean_url);
                let rt = tokio::runtime::Runtime::new();
                let res = if let Ok(rt) = rt {
                    rt.block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(6))
                            .build()
                            .map_err(|e| e.to_string())?;
                        let resp = client.get(&ping_url).send().await
                            .map_err(|e| format!("Tidak dapat terhubung: {}", e))?;
                        let st = resp.status();
                        if st.is_success() || st.as_u16() == 401 {
                            Ok(format!("Terhubung ke peladen (HTTP {})", st.as_u16()))
                        } else {
                            Err(format!("Peladen merespons status {}", st.as_u16()))
                        }
                    })
                } else {
                    Err("Runtime async gagal diinisialisasi".into())
                };

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak2.upgrade() {
                        app.set_is_testing_wizard(false);
                        match res {
                            Ok(msg) => {
                                app.set_wizard_test_message(format!("✅ {}", msg).into());
                                app.set_wizard_test_success(true);
                            }
                            Err(e) => {
                                app.set_wizard_test_message(format!("❌ Sambungan gagal: {}", e).into());
                                app.set_wizard_test_success(false);
                            }
                        }
                    }
                });
            });
        });
    }

    // Setup Wizard Onboarding: Selesaikan Penyiapan Awal
    {
        let conn_arc = Arc::clone(&conn_arc);
        let app_weak = app_weak.clone();
        app.on_finish_wizard(move |url_slint| {
            let mut raw_url = url_slint.to_string().trim().to_string();
            if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
                raw_url = format!("http://{}", raw_url);
            }
            let clean_url = raw_url.trim_end_matches('/').to_string();

            if let Ok(conn) = conn_arc.lock() {
                let _ = conn.execute(
                    "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_url', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
                    [&clean_url],
                );
            }

            if let Some(app) = app_weak.upgrade() {
                app.set_api_endpoint_url(clean_url.clone().into());
                app.set_sync_base_url(clean_url.into());
                app.set_is_fresh_install(false);
                app.set_sync_toast_message("Konfigurasi awal berhasil diselesaikan!".into());
                app.set_sync_toast_is_error(false);
                refresh_sync_status_ui(&app, &conn_arc);
            }
        });
    }

    // Periksa Pembaruan Sistem (Check for Updates Callback)
    {
        let app_weak = app_weak.clone();
        app.on_check_for_updates(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_update_status_text("Sedang memeriksa versi terbaru dari repositori resmi...".into());
            }
            check_updates_background(app_weak.clone());
        });
    }

    // Setup Background Autosync & Heartbeat Connection Checker
    {
        let app_weak_sync = app_weak.clone();
        let conn_arc_sync = Arc::clone(&conn_arc);
        std::thread::spawn(move || {
            // Jeda 2 detik saat startup agar jendela UI telah ter-render penuh
            std::thread::sleep(std::time::Duration::from_secs(2));

            let mut last_pull_instant = std::time::Instant::now();
            let mut is_first_run = true;

            loop {
                let (api_url, api_token) = {
                    if let Ok(conn) = conn_arc_sync.lock() {
                        let u: String = conn.query_row("SELECT value FROM settings WHERE key = 'api_url';", [], |r| r.get(0)).unwrap_or_default();
                        let t: String = conn.query_row(
                            "SELECT value FROM settings WHERE key = 'api_key' UNION SELECT value FROM settings WHERE key = 'api_token' UNION SELECT token FROM user_tokens ORDER BY expires_at DESC LIMIT 1;",
                            [],
                            |r| r.get(0)
                        ).unwrap_or_default();
                        (u, t)
                    } else {
                        ("".into(), "".into())
                    }
                };

                let target_url = if api_url.trim().is_empty() {
                    "http://sumbangan.smage.my.id".to_string()
                } else {
                    api_url.trim().to_string()
                };

                let ping_url = format!("{}/api/v1/ping", target_url.trim_end_matches('/'));
                let rt = tokio::runtime::Runtime::new();
                let server_online = if let Ok(ref rt) = rt {
                    rt.block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(5))
                            .build();
                        if let Ok(c) = client {
                            let mut req = c.get(&ping_url);
                            if !api_token.is_empty() {
                                req = req.header("X-API-Key", &api_token);
                            }
                            req.send()
                                .await
                                .map(|resp| resp.status().is_success() || resp.status().as_u16() == 401)
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    })
                } else {
                    false
                };

                let app_weak_cb = app_weak_sync.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(app) = app_weak_cb.upgrade() {
                        app.set_is_online(server_online);
                    }
                });

                if server_online && !api_token.is_empty() {
                        let pending_count: i64 = {
                            if let Ok(conn) = conn_arc_sync.lock() {
                                let p1: i64 = conn.query_row("SELECT count(*) FROM payments WHERE sync_status = 'pending_push';", [], |r| r.get(0)).unwrap_or(0);
                                let p2: i64 = conn.query_row("SELECT count(*) FROM savings_transactions WHERE sync_status = 'pending_push';", [], |r| r.get(0)).unwrap_or(0);
                                p1 + p2
                            } else { 0 }
                        };

                        let should_sync = is_first_run || pending_count > 0 || last_pull_instant.elapsed().as_secs() >= 300;

                        if should_sync {
                            is_first_run = false;
                            last_pull_instant = std::time::Instant::now();

                            let db_state = crate::db::DbState {
                                conn: Mutex::new(crate::db::init_db_standalone().unwrap()),
                            };
                            let sync_res = if let Ok(ref rt) = rt {
                                rt.block_on(async {
                                    crate::commands::perform_sync(&db_state).await
                                })
                            } else {
                                Err("Runtime error".into())
                            };

                            let app_weak_res = app_weak_sync.clone();
                            let conn_arc_res = Arc::clone(&conn_arc_sync);
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(app) = app_weak_res.upgrade() {
                                    match sync_res {
                                        Ok(msg) => {
                                            app.set_sync_message(msg.into());
                                            app.set_sync_toast_message("Autosync Berhasil: Data cloud & lokal selaras.".into());
                                            app.set_sync_toast_is_error(false);
                                            app.set_is_online(true);
                                            if let Ok(conn) = conn_arc_res.lock() {
                                                update_branding_and_settings(&app, &conn);
                                            }
                                            refresh_sync_status_ui(&app, &conn_arc_res);
                                            refresh_today_payments_ui(&app, &conn_arc_res);
                                        }
                                        Err(e) => {
                                            println!("[WARN] Autosync background: {}", e);
                                        }
                                    }
                                }
                            });
                        }
                    }

                std::thread::sleep(std::time::Duration::from_secs(30));
            }
        });
    }

    // Jalankan Event Loop GUI Slint Native
    app.run()?;
    Ok(())
}

// ==========================================
// UI DATA POPULATORS & HELPERS
// ==========================================

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let clean: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = clean.as_bytes();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let decode_char = |b: u8| -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let b0 = decode_char(chunk[0])?;
        let b1 = decode_char(chunk[1])?;
        out.push((b0 << 2) | (b1 >> 4));
        if chunk[2] != b'=' {
            let b2 = decode_char(chunk[2])?;
            out.push((b1 << 4) | (b2 >> 2));
            if chunk[3] != b'=' {
                let b3 = decode_char(chunk[3])?;
                out.push((b2 << 6) | b3);
            }
        }
    }
    Some(out)
}

fn load_and_cache_school_logo(conn: &Connection) -> Option<std::path::PathBuf> {
    let logo_data: String = conn.query_row(
        "SELECT value FROM settings WHERE key IN ('logo_left', 'app_logo') AND value != '' ORDER BY CASE WHEN key = 'logo_left' THEN 1 ELSE 2 END LIMIT 1;",
        [],
        |r| r.get(0)
    ).unwrap_or_default();

    let cache_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("me.ppti.partisipasi-sekolah");
    let _ = std::fs::create_dir_all(&cache_dir);
    let logo_file = cache_dir.join("cached_school_logo.png");

    if !logo_data.is_empty() {
        if let Some(pos) = logo_data.find("base64,") {
            let b64_str = &logo_data[pos + 7..];
            if let Some(bytes) = base64_decode(b64_str) {
                let _ = std::fs::write(&logo_file, &bytes);
                return Some(logo_file);
            }
        }
    }

    if logo_file.exists() {
        Some(logo_file)
    } else {
        None
    }
}

fn update_branding_and_settings(app: &AppWindow, conn: &Connection) {
    let school_name: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'school_name';",
        [],
        |r| r.get(0)
    ).unwrap_or_default();

    let win_title = if !school_name.trim().is_empty() {
        format!("{} - Partisipasi Sekolah Desktop v0.2.0", school_name.trim())
    } else {
        "Partisipasi Sekolah Desktop v0.2.0".to_string()
    };
    app.set_window_title(win_title.into());

    let savings_enabled: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'savings_enabled';",
        [],
        |r| r.get(0)
    ).unwrap_or_else(|_| "0".to_string());
    app.set_savings_enabled(savings_enabled.trim() == "1");

    if let Some(logo_path) = load_and_cache_school_logo(conn) {
        if let Ok(img) = slint::Image::load_from_path(&logo_path) {
            app.set_has_school_logo(true);
            app.set_school_logo(img);
        }
    }
}


fn populate_receipt_modal(
    app: &AppWindow,
    conn: &Connection,
    trx_code: &str,
    cashier_name: &str,
) -> Result<(), String> {
    // 1. Query data transaksi & siswa
    let (payment_id, created_at, payment_method_name, total_amount, notes, student_nama, student_nis, student_kelas) = conn.query_row(
        "SELECT p.id, p.created_at, COALESCE(p.payment_method_name, 'Tunai'), p.total_amount, p.notes, s.nama, s.nis, COALESCE(s.kelas, c.name, '-')
         FROM payments p
         JOIN students s ON p.student_id = s.id
         LEFT JOIN classrooms c ON s.classroom_id = c.id
         WHERE p.transaction_code = ?1;",
        [trx_code],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
            ))
        }
    ).map_err(|e| format!("Transaksi tidak ditemukan: {}", e))?;

    // 2. Query rincian pembayaran
    let mut stmt = conn.prepare(
        "SELECT t.name, t.type, a.month, d.amount, y.name
         FROM payment_details d
         JOIN payment_assignments a ON d.payment_assignment_id = a.id
         JOIN payment_types t ON a.payment_type_id = t.id
         LEFT JOIN academic_years y ON a.academic_year_id = y.id
         WHERE d.payment_id = ?1;"
    ).map_err(|e| e.to_string())?;

    let items_raw: Vec<(String, Option<i32>, f64, Option<String>)> = stmt.query_map([&payment_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    }).map_err(|e| e.to_string())?.flatten().collect();

    let month_name = |m: i32| -> &'static str {
        match m {
            7 => "Juli", 8 => "Agustus", 9 => "September", 10 => "Oktober",
            11 => "November", 12 => "Desember", 1 => "Januari", 2 => "Februari",
            3 => "Maret", 4 => "April", 5 => "Mei", 6 => "Juni", _ => "",
        }
    };

    let mut modal_items = Vec::new();
    for (i, item) in items_raw.iter().enumerate() {
        let mut desc = item.0.clone();
        if let Some(m) = item.1 {
            let m_name = month_name(m);
            if !m_name.is_empty() {
                desc.push_str(&format!(" ({})", m_name));
            }
        }
        if let Some(ref y) = item.3 {
            desc.push_str(&format!(" [T.A {}]", y));
        }
        modal_items.push(ReceiptModalItem {
            index: (i + 1) as i32,
            description: desc.into(),
            amount: format_idr(item.2).into(),
        });
    }

    let school_name: String = conn.query_row("SELECT value FROM settings WHERE key = 'school_name';", [], |r| r.get(0)).unwrap_or_else(|_| "Nama Sekolah".into());
    let school_address: String = conn.query_row("SELECT value FROM settings WHERE key = 'school_address';", [], |r| r.get(0)).unwrap_or_else(|_| "Alamat Sekolah".into());

    app.set_receipt_modal_data(ReceiptModalData {
        transaction_code: trx_code.into(),
        created_at: created_at.into(),
        student_name: student_nama.into(),
        student_nis: student_nis.unwrap_or_else(|| "-".into()).into(),
        student_class: student_kelas.into(),
        payment_method: payment_method_name.into(),
        total_amount: format_idr(total_amount).into(),
        cashier_name: if cashier_name.is_empty() { "Petugas Kasir".into() } else { cashier_name.into() },
        notes: notes.unwrap_or_default().into(),
        school_name: school_name.into(),
        school_address: school_address.into(),
    });
    app.set_receipt_modal_items(Rc::new(VecModel::from(modal_items)).into());
    app.set_show_receipt_modal(true);

    Ok(())
}

fn send_receipt_to_printer(conn: &Connection, trx_code: &str, cashier_name: &str) -> Result<(), String> {
    // 1. Query data transaksi
    let (payment_id, created_at, payment_method_name, total_amount, notes, student_nama, student_nis, student_kelas) = conn.query_row(
        "SELECT p.id, p.created_at, COALESCE(p.payment_method_name, 'Tunai'), p.total_amount, p.notes, s.nama, s.nis, COALESCE(s.kelas, c.name, '-')
         FROM payments p
         JOIN students s ON p.student_id = s.id
         LEFT JOIN classrooms c ON s.classroom_id = c.id
         WHERE p.transaction_code = ?1;",
        [trx_code],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
            ))
        }
    ).map_err(|e| format!("Transaksi tidak ditemukan: {}", e))?;

    let school_name: String = conn.query_row("SELECT value FROM settings WHERE key = 'school_name';", [], |r| r.get(0)).unwrap_or_else(|_| "Nama Sekolah".into());
    let school_address: String = conn.query_row("SELECT value FROM settings WHERE key = 'school_address';", [], |r| r.get(0)).unwrap_or_else(|_| "".into());

    let mut stmt = conn.prepare(
        "SELECT t.name, a.month, d.amount, y.name
         FROM payment_details d
         JOIN payment_assignments a ON d.payment_assignment_id = a.id
         JOIN payment_types t ON a.payment_type_id = t.id
         LEFT JOIN academic_years y ON a.academic_year_id = y.id
         WHERE d.payment_id = ?1;"
    ).map_err(|e| e.to_string())?;

    let month_name = |m: i32| -> &'static str {
        match m {
            7 => "Juli", 8 => "Agustus", 9 => "September", 10 => "Oktober",
            11 => "November", 12 => "Desember", 1 => "Januari", 2 => "Februari",
            3 => "Maret", 4 => "April", 5 => "Mei", 6 => "Juni", _ => "",
        }
    };

    let items: Vec<(String, Option<i32>, f64, Option<String>)> = stmt.query_map([&payment_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<i32>>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    }).map_err(|e| e.to_string())?.flatten().collect();

    // Dokumen Struk Monospace Standar Printer Kasir (Thermal POS / Dot Matrix)
    let mut txt = String::new();
    txt.push_str("========================================\n");
    txt.push_str(&format!("{:^40}\n", school_name));
    if !school_address.is_empty() {
        txt.push_str(&format!("{:^40}\n", school_address));
    }
    txt.push_str("========================================\n");
    txt.push_str(&format!("{:^40}\n", "KWITANSI PEMBAYARAN"));
    txt.push_str("----------------------------------------\n");
    txt.push_str(&format!("No. Transaksi : {}\n", trx_code));
    txt.push_str(&format!("Tanggal       : {}\n", created_at));
    txt.push_str(&format!("Nama Siswa    : {}\n", student_nama));
    txt.push_str(&format!("NIS / Kelas   : {} / {}\n", student_nis.unwrap_or_else(|| "-".into()), student_kelas));
    txt.push_str(&format!("Metode Bayar  : {}\n", payment_method_name));
    txt.push_str("----------------------------------------\n");
    txt.push_str(&format!("{:<3}{:<25}{:>12}\n", "No", "Keterangan", "Jumlah"));
    txt.push_str("----------------------------------------\n");
    for (i, itm) in items.iter().enumerate() {
        let mut desc = itm.0.clone();
        if let Some(m) = itm.1 {
            let mn = month_name(m);
            if !mn.is_empty() { desc.push_str(&format!(" ({})", mn)); }
        }
        if desc.len() > 24 { desc.truncate(24); }
        txt.push_str(&format!("{:<3}{:<25}{:>12}\n", i + 1, desc, format_idr(itm.2)));
    }
    txt.push_str("----------------------------------------\n");
    txt.push_str(&format!("{:<28}{:>12}\n", "TOTAL PEMBAYARAN:", format_idr(total_amount)));
    txt.push_str("----------------------------------------\n");
    if let Some(ref n) = notes {
        if !n.is_empty() {
            txt.push_str(&format!("Catatan: {}\n", n));
        }
    }
    txt.push_str(&format!("Kasir: {}\n", if cashier_name.is_empty() { "Petugas Kasir" } else { cashier_name }));
    txt.push_str("========================================\n");
    txt.push_str(&format!("{:^40}\n", "Bukti Pembayaran yang Sah"));
    txt.push_str("\n\n\n\n\x1b\x69"); // Feed and Paper cut command

    let tmp_path = std::env::temp_dir().join(format!("kwitansi_{}.txt", trx_code));
    std::fs::write(&tmp_path, txt).map_err(|e| format!("Gagal buat spooler kwitansi: {}", e))?;

    // Jalankan adapter pencetakan native CUPS (lpr) langsung tanpa membuka web browser
    #[cfg(unix)]
    {
        let output = std::process::Command::new("lpr")
            .arg(&tmp_path)
            .output()
            .map_err(|e| format!("Perintah lpr gagal: {}", e))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Adapter printer CUPS: {}", err_msg.trim()));
        }
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "print", tmp_path.to_str().unwrap_or("")])
            .spawn();
    }

    Ok(())
}

fn get_active_user_id(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM users LIMIT 1;", [], |r| r.get(0))
        .unwrap_or_else(|_| "local-cashier".to_string())
}

fn execute_savings_transaction(
    conn: &Connection,
    student_id: &str,
    tx_type: &str,
    amount: f64,
    notes: &str,
) -> Result<f64, String> {
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let user_id = get_active_user_id(conn);

    let savings_id: String = conn.query_row(
        "SELECT id FROM savings WHERE student_id = ?1;",
        [student_id],
        |r| r.get(0)
    ).unwrap_or_else(|_| {
        let new_id = Uuid::new_v4().to_string();
        let _ = conn.execute(
            "INSERT INTO savings (id, student_id, balance, is_active, created_at, updated_at, sync_status) 
             VALUES (?1, ?2, 0.0, 1, ?3, ?3, 'pending_push');",
            params![new_id, student_id, now_str]
        );
        new_id
    });

    let current_balance: f64 = conn.query_row(
        "SELECT balance FROM savings WHERE id = ?1;",
        [&savings_id],
        |r| r.get(0)
    ).unwrap_or(0.0);

    let new_balance = match tx_type {
        "setor" | "deposit" => current_balance + amount,
        "tarik" | "withdraw" => {
            if current_balance < amount {
                return Err("Saldo tabungan tidak mencukupi untuk penarikan!".to_string());
            }
            current_balance - amount
        }
        _ => return Err("Tipe transaksi tidak valid".to_string()),
    };

    let trx_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO savings_transactions (id, savings_id, type, amount, balance_after, notes, user_id, created_at, sync_status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending_push');",
        params![trx_id, savings_id, tx_type, amount, new_balance, notes, user_id, now_str]
    ).map_err(|e| format!("Gagal mencatat mutasi tabungan: {}", e))?;

    conn.execute(
        "UPDATE savings SET balance = ?1, updated_at = ?2, sync_status = 'pending_push' WHERE id = ?3;",
        params![new_balance, now_str, savings_id]
    ).map_err(|e| format!("Gagal update saldo tabungan: {}", e))?;

    Ok(new_balance)
}

fn perform_student_search(
    app: &AppWindow,
    conn_arc: &Arc<Mutex<Connection>>,
    state: &Arc<Mutex<SlintAppState>>,
    query: &str,
) {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        app.set_show_search_results(false);
        app.set_search_results(Rc::new(VecModel::default()).into());
        return;
    }

    let filter_status = state.lock().unwrap().student_filter_status;
    let active_val = if filter_status == 1 { 0 } else { 1 };

    if let Ok(conn) = conn_arc.lock() {
        let q = format!("%{}%", trimmed);
        let mut list = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT s.id, s.nama, s.nis, s.nisn, c.name as kelas, COALESCE(s.is_active, 1) as is_act 
             FROM students s
             LEFT JOIN classrooms c ON s.classroom_id = c.id
             WHERE COALESCE(s.is_active, 1) = ?1 
               AND (s.nama LIKE ?2 OR s.nis LIKE ?2 OR s.nisn LIKE ?2)
             ORDER BY s.nama ASC
             LIMIT 12;"
        ) {
            if let Ok(rows) = stmt.query_map(rusqlite::params![active_val, &q], |row| {
                Ok(StudentSearchResult {
                    id: row.get::<_, String>(0)?.into(),
                    nama: row.get::<_, String>(1)?.into(),
                    nis: row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "-".into()).into(),
                    nisn: row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "-".into()).into(),
                    kelas: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| "-".into()).into(),
                    is_active: row.get::<_, i32>(5).unwrap_or(1) == 1,
                })
            }) {
                for item in rows.flatten() {
                    list.push(item);
                }
            }
        }

        let has_results = !list.is_empty();
        app.set_search_results(Rc::new(VecModel::from(list)).into());
        app.set_show_search_results(has_results);
    }
}

fn load_student_details_ui(
    app: &AppWindow,
    conn_arc: &Arc<Mutex<Connection>>,
    state: &Arc<Mutex<SlintAppState>>,
    student_id: &str,
    reset_collapse: bool,
) {
    if let Ok(conn) = conn_arc.lock() {
        // 1. Data Siswa
        let mut s_data = StudentData {
            id: student_id.into(),
            nis: "-".into(),
            nisn: "-".into(),
            nama: "Tidak Ditemukan".into(),
            kelas: "-".into(),
            savings_balance: "Rp 0".into(),
        };

        if let Ok(mut stmt) = conn.prepare(
            "SELECT s.nis, s.nisn, s.nama, c.name 
             FROM students s
             LEFT JOIN classrooms c ON s.classroom_id = c.id
             WHERE s.id = ?1;"
        ) {
            if let Ok(mut rows) = stmt.query([student_id]) {
                if let Ok(Some(row)) = rows.next() {
                    let nis: Option<String> = row.get(0).unwrap_or(None);
                    let nisn: Option<String> = row.get(1).unwrap_or(None);
                    let nama: String = row.get(2).unwrap_or_default();
                    let kelas: Option<String> = row.get(3).unwrap_or(None);

                    s_data.nis = nis.unwrap_or_else(|| "-".into()).into();
                    s_data.nisn = nisn.unwrap_or_else(|| "-".into()).into();
                    s_data.nama = nama.into();
                    s_data.kelas = kelas.unwrap_or_else(|| "-".into()).into();
                }
            }
        }

        // Ambil Saldo Tabungan dari tabel savings
        let bal: f64 = conn.query_row(
            "SELECT balance FROM savings WHERE student_id = ?1;",
            [student_id],
            |r| r.get(0)
        ).unwrap_or(0.0);
        s_data.savings_balance = format_idr(bal).into();
        app.set_current_student(s_data);

        // 2. Data Assignments & Summary (Dikelompokkan per Tahun Ajaran & Pos Bayar)
        let mut assignments_vec = Vec::new();
        let mut total_tagihan = 0.0;
        let mut total_dibayar = 0.0;
        let mut total_keringanan = 0.0;
        let mut total_tunggakan = 0.0;

        if let Ok(mut stmt) = conn.prepare(
            "SELECT a.id, t.name, t.type, a.month, a.amount, a.paid_amount, a.relief_amount, a.status, COALESCE(y.name, 'Lainnya') as academic_year_name, COALESCE(y.is_active, 0) as is_active
             FROM payment_assignments a
             JOIN payment_types t ON a.payment_type_id = t.id
             LEFT JOIN academic_years y ON a.academic_year_id = y.id
             WHERE a.student_id = ?1
             ORDER BY COALESCE(y.name, '0000') DESC, t.name ASC, a.month ASC;"
        ) {
            if let Ok(rows) = stmt.query_map([student_id], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let t_type: String = row.get(2)?;
                let month: Option<i32> = row.get(3)?;
                let amount: f64 = row.get(4)?;
                let paid: f64 = row.get(5)?;
                let relief: f64 = row.get(6)?;
                let status: String = row.get(7)?;
                let year_name: String = row.get(8)?;
                let is_active: i32 = row.get(9)?;

                let month_name = match month {
                    Some(7) => "Juli", Some(8) => "Agustus", Some(9) => "September",
                    Some(10) => "Oktober", Some(11) => "November", Some(12) => "Desember",
                    Some(1) => "Januari", Some(2) => "Februari", Some(3) => "Maret",
                    Some(4) => "April", Some(5) => "Mei", Some(6) => "Juni",
                    _ => "",
                };

                let remaining = (amount - paid - relief).max(0.0);

                Ok((id, name, t_type, month_name.to_string(), amount, paid, relief, remaining, status, year_name, is_active))
            }) {
                let all_rows: Vec<_> = rows.flatten().collect();
                let mut year_stats: std::collections::HashMap<String, (i32, f64)> = std::collections::HashMap::new();

                for r in &all_rows {
                    total_tagihan += r.4;
                    total_dibayar += r.5;
                    total_keringanan += r.6;
                    total_tunggakan += r.7;

                    let stat = year_stats.entry(r.9.clone()).or_insert((0, 0.0));
                    stat.0 += 1;
                    stat.1 += r.7; // remaining
                }

                // Reset status collapsible jika diminta: TP aktif dibuka, TP lama diciutkan (default)
                if reset_collapse {
                    let mut st = state.lock().unwrap();
                    st.collapsed_years.clear();
                    for r in &all_rows {
                        if r.10 != 1 {
                            st.collapsed_years.insert(r.9.clone());
                        }
                    }
                }

                let mut current_year: Option<String> = None;
                for r in all_rows {
                    let year_name = r.9.clone();
                    let is_active = r.10 == 1;

                    // Sisipkan Header Grup Tahun Pelajaran jika berganti
                    if current_year.as_ref() != Some(&year_name) {
                        current_year = Some(year_name.clone());
                        let (count_bills, rem_sum) = year_stats.get(&year_name).cloned().unwrap_or((0, 0.0));
                        let is_collapsed = state.lock().unwrap().collapsed_years.contains(&year_name);

                        let header_title = if is_active {
                            format!("Tahun Pelajaran {} (Aktif)", year_name)
                        } else {
                            format!("Tahun Pelajaran {}", year_name)
                        };

                        let rem_text = if rem_sum > 0.0 {
                            format!("Sisa: {}", format_idr(rem_sum))
                        } else {
                            "Lunas".to_string()
                        };

                        assignments_vec.push(AssignmentData {
                            id: format!("hdr-{}", year_name).into(),
                            academic_year_name: year_name.clone().into(),
                            payment_type_name: "".into(),
                            payment_type_type: "".into(),
                            month_name: "".into(),
                            amount: "".into(),
                            paid_amount: "".into(),
                            remaining_amount: "".into(),
                            remaining_raw: 0,
                            status: "".into(),
                            is_selected: false,
                            is_group_header: true,
                            group_header_title: header_title.into(),
                            is_active_year: is_active,
                            is_collapsed,
                            is_hidden: false,
                            total_bills_in_year: count_bills,
                            total_remaining_in_year: rem_text.into(),
                        });
                    }

                    let is_collapsed = state.lock().unwrap().collapsed_years.contains(&year_name);
                    if !is_collapsed {
                        let is_sel = state.lock().unwrap().cart.iter().any(|c| c.assignment_id == r.0);

                        assignments_vec.push(AssignmentData {
                            id: r.0.into(),
                            academic_year_name: year_name.into(),
                            payment_type_name: r.1.into(),
                            payment_type_type: r.2.into(),
                            month_name: r.3.into(),
                            amount: format_idr(r.4).into(),
                            paid_amount: format_idr(r.5).into(),
                            remaining_amount: format_idr(r.7).into(),
                            remaining_raw: r.7 as i32,
                            status: r.8.into(),
                            is_selected: is_sel,
                            is_group_header: false,
                            group_header_title: "".into(),
                            is_active_year: is_active,
                            is_collapsed: false,
                            is_hidden: false,
                            total_bills_in_year: 0,
                            total_remaining_in_year: "".into(),
                        });
                    }
                }
            }
        }

        app.set_assignments(Rc::new(VecModel::from(assignments_vec)).into());
        app.set_bill_summary(BillSummaryData {
            total_tagihan: format_idr(total_tagihan).into(),
            total_dibayar: format_idr(total_dibayar).into(),
            total_keringanan: format_idr(total_keringanan).into(),
            total_tunggakan: format_idr(total_tunggakan).into(),
        });

        // 3. Muat Riwayat Tabungan Siswa dari tabel savings_transactions
        let mut sav_hist = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT st.id, st.type, st.amount, st.balance_after, st.notes, st.created_at 
             FROM savings_transactions st 
             JOIN savings sv ON st.savings_id = sv.id
             WHERE sv.student_id = ?1 
             ORDER BY st.created_at DESC LIMIT 30;"
        ) {
            if let Ok(rows) = stmt.query_map([student_id], |row| {
                let id: String = row.get(0)?;
                let m_type: String = row.get(1)?;
                let amt: f64 = row.get(2)?;
                let bal_after: f64 = row.get(3)?;
                let notes: Option<String> = row.get(4)?;
                let created: String = row.get(5)?;
                Ok(SavingsHistoryData {
                    id: id.into(),
                    transaction_type: m_type.into(),
                    amount: format_idr(amt).into(),
                    balance_after: format_idr(bal_after).into(),
                    notes: notes.unwrap_or_else(|| "-".into()).into(),
                    date: created.into(),
                })
            }) {
                for r in rows.flatten() {
                    sav_hist.push(r);
                }
            }
        }
        app.set_savings_history(Rc::new(VecModel::from(sav_hist)).into());
    }
}

fn update_cart_and_selection_ui(app: &AppWindow, state: &SlintAppState) {
    let mut cart_slint = Vec::new();
    let mut total = 0.0;

    for item in &state.cart {
        total += item.pay_amount;
        cart_slint.push(CartItemData {
            assignment_id: item.assignment_id.clone().into(),
            name: item.name.clone().into(),
            remaining_text: format_idr(item.remaining).into(),
            pay_amount_text: format_idr(item.pay_amount).into(),
        });
    }

    app.set_cart_items(Rc::new(VecModel::from(cart_slint)).into());
    app.set_cart_total(format_idr(total).into());

    // Update checkbox is_selected di assignment list
    let assignments = app.get_assignments();
    let mut updated_assignments = Vec::new();
    for i in 0..assignments.row_count() {
        if let Some(mut item) = assignments.row_data(i) {
            if !item.is_group_header {
                item.is_selected = state.cart.iter().any(|c| c.assignment_id == item.id.as_str());
            }
            updated_assignments.push(item);
        }
    }
    app.set_assignments(Rc::new(VecModel::from(updated_assignments)).into());
}

fn refresh_today_payments_ui(app: &AppWindow, conn_arc: &Arc<Mutex<Connection>>) {
    if let Ok(conn) = conn_arc.lock() {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let month = Local::now().format("%Y-%m").to_string();
        let q_today = format!("{}%", today);
        let q_month = format!("{}%", month);

        let today_total: f64 = conn.query_row(
            "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments WHERE created_at LIKE ?1;",
            [&q_today],
            |r| r.get(0)
        ).unwrap_or(0.0);

        let monthly_total: f64 = conn.query_row(
            "SELECT COALESCE(SUM(total_amount), 0.0) FROM payments WHERE created_at LIKE ?1;",
            [&q_month],
            |r| r.get(0)
        ).unwrap_or(0.0);

        let today_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM payments WHERE created_at LIKE ?1;",
            [&q_today],
            |r| r.get(0)
        ).unwrap_or(0);

        app.set_today_cashier_total(format_idr(today_total).into());
        app.set_monthly_cashier_total(format_idr(monthly_total).into());
        app.set_today_cashier_count(format!("{} Transaksi", today_count).into());

        let mut list = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT p.transaction_code, s.nama, p.total_amount, COALESCE(p.payment_method_name, p.payment_method_id, 'Tunai'), p.created_at 
             FROM payments p
             JOIN students s ON p.student_id = s.id
             WHERE p.created_at LIKE ?1
             ORDER BY p.created_at DESC LIMIT 30;"
        ) {
            if let Ok(rows) = stmt.query_map([&q_today], |row| {
                let code: String = row.get(0)?;
                let name: String = row.get(1)?;
                let amt: f64 = row.get(2)?;
                let meth: Option<String> = row.get(3)?;
                let created: String = row.get(4)?;

                let m_label = match meth.as_deref() {
                    Some(s) if s.eq_ignore_ascii_case("transfer") || s.contains("Transfer") => "Transfer",
                    Some(s) if s.eq_ignore_ascii_case("tabungan") || s.contains("Tabungan") => "Tabungan",
                    _ => "Tunai",
                };

                let time_only = created.split(' ').nth(1).unwrap_or(&created).to_string();

                Ok(TodayPaymentData {
                    transaction_code: code.into(),
                    student_name: name.into(),
                    total_amount: format_idr(amt).into(),
                    payment_method: m_label.into(),
                    created_at: time_only.into(),
                })
            }) {
                for r in rows.flatten() {
                    list.push(r);
                }
            }
        }
        app.set_today_payments(Rc::new(VecModel::from(list)).into());
    }
}

fn refresh_history_tab_ui(app: &AppWindow, conn_arc: &Arc<Mutex<Connection>>, search: &str) {
    if let Ok(conn) = conn_arc.lock() {
        let mut list = Vec::new();
        let q = format!("%{}%", search.trim());

        if let Ok(mut stmt) = conn.prepare(
            "SELECT p.id, p.transaction_code, s.nama, COALESCE(s.kelas, c.name, '-'), p.total_amount, COALESCE(p.payment_method_name, 'Tunai'), p.created_at, COALESCE(p.sync_status, 'synced'), COALESCE(p.notes, '')
             FROM payments p
             JOIN students s ON p.student_id = s.id
             LEFT JOIN classrooms c ON s.classroom_id = c.id
             WHERE (p.transaction_code LIKE ?1 OR s.nama LIKE ?1)
             ORDER BY p.created_at DESC LIMIT 100;"
        ) {
            if let Ok(rows) = stmt.query_map([&q], |row| {
                let id: String = row.get(0)?;
                let code: String = row.get(1)?;
                let name: String = row.get(2)?;
                let kelas: String = row.get(3)?;
                let amt: f64 = row.get(4)?;
                let meth: String = row.get(5)?;
                let created: String = row.get(6)?;
                let sync_status: String = row.get(7)?;
                let notes: String = row.get(8)?;

                let m_label = match meth.as_str() {
                    s if s.eq_ignore_ascii_case("transfer") || s.contains("Transfer") => "Transfer",
                    s if s.eq_ignore_ascii_case("tabungan") || s.contains("Tabungan") => "Tabungan",
                    _ => "Tunai",
                };

                Ok(HistoryRecord {
                    id: id.into(),
                    transaction_code: code.into(),
                    student_name: name.into(),
                    student_class: kelas.into(),
                    payment_method: m_label.into(),
                    total_amount: format_idr(amt).into(),
                    notes: notes.into(),
                    created_at: created.into(),
                    sync_status: sync_status.into(),
                })
            }) {
                for r in rows.flatten() {
                    list.push(r);
                }
            }
        }
        app.set_history_records(Rc::new(VecModel::from(list)).into());
    }
}

fn refresh_reports_tab_ui(app: &AppWindow, conn_arc: &Arc<Mutex<Connection>>) {
    if let Ok(conn) = conn_arc.lock() {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let q = format!("{}%", today);

        let mut grand_total = 0.0;
        let mut cash_total = 0.0;
        let mut transfer_total = 0.0;
        let mut savings_total = 0.0;

        if let Ok(mut stmt) = conn.prepare(
            "SELECT payment_method_id, SUM(total_amount) 
             FROM payments 
             WHERE created_at LIKE ?1 
             GROUP BY payment_method_id;"
        ) {
            if let Ok(rows) = stmt.query_map([&q], |row| {
                let meth: String = row.get(0)?;
                let sum_amt: f64 = row.get(1)?;
                Ok((meth, sum_amt))
            }) {
                for r in rows.flatten() {
                    grand_total += r.1;
                    match r.0.as_str() {
                        "cash" => cash_total += r.1,
                        "transfer" => transfer_total += r.1,
                        "tabungan" => savings_total += r.1,
                        _ => {}
                    }
                }
            }
        }

        app.set_grand_total(format_idr(grand_total).into());
        app.set_cash_total(format_idr(cash_total).into());
        app.set_transfer_total(format_idr(transfer_total).into());
        app.set_savings_total(format_idr(savings_total).into());

        // Rekap per Kasir
        let mut summaries = Vec::new();
        summaries.push(CashierSummaryRecord {
            cashier_name: "Loket 1 (Aktif)".into(),
            cash_amount: format_idr(cash_total).into(),
            transfer_amount: format_idr(transfer_total).into(),
            savings_amount: format_idr(savings_total).into(),
            total_amount: format_idr(grand_total).into(),
        });
        app.set_cashier_summaries(Rc::new(VecModel::from(summaries)).into());
    }
}

fn refresh_sync_status_ui(app: &AppWindow, conn_arc: &Arc<Mutex<Connection>>) {
    if let Ok(conn) = conn_arc.lock() {
        let pending_payments: i64 = conn.query_row(
            "SELECT count(*) FROM payments WHERE sync_status = 'pending_push';", [], |r| r.get(0)
        ).unwrap_or(0);

        let pending_savings: i64 = conn.query_row(
            "SELECT count(*) FROM savings_transactions WHERE sync_status = 'pending_push';", [], |r| r.get(0)
        ).unwrap_or(0);

        let total_pending = pending_payments + pending_savings;
        app.set_pending_sync_count(total_pending.to_string().into());
        app.set_sync_pending_count(total_pending.to_string().into());

        // Muat Config Server
        if let Ok(mut stmt) = conn.prepare("SELECT key, value FROM settings WHERE key IN ('api_url', 'api_key', 'api_token', 'last_synced_at');") {
            if let Ok(rows) = stmt.query_map([], |row| {
                let k: String = row.get(0)?;
                let v: String = row.get(1)?;
                Ok((k, v))
            }) {
                for r in rows.flatten() {
                    match r.0.as_str() {
                        "api_url" => app.set_sync_base_url(r.1.into()),
                        "api_key" | "api_token" => {
                            if !r.1.is_empty() {
                                app.set_sync_api_token(r.1.into());
                            }
                        }
                        "last_synced_at" => {
                            if r.1.is_empty() || r.1.starts_with("1970") {
                                app.set_sync_last_time("Belum pernah sinkron".into());
                            } else {
                                app.set_sync_last_time(r.1.into());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn process_deep_link_url(
    raw_url: &str,
    app: &AppWindow,
    conn_arc: &Arc<Mutex<Connection>>,
    state: &Arc<Mutex<SlintAppState>>,
) {
    if !raw_url.starts_with("partisipasi-sekolah://") && !raw_url.starts_with("partisipasi://") {
        return;
    }
    println!("[INFO] Memproses deep-link otorisasi: {}", raw_url);
    let query_str = raw_url.split_once('?').map(|x| x.1).unwrap_or("");
    let mut token = String::new();
    let mut nama = String::new();
    let mut role = String::new();
    let mut code = String::new();

    for pair in query_str.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let decoded = urlencoding::decode(v).unwrap_or_else(|_| v.into()).to_string();
            match k {
                "token" => token = decoded,
                "nama" => nama = decoded,
                "role" => role = decoded,
                "code" => code = decoded,
                _ => {}
            }
        }
    }

    if !token.is_empty() {
        if let Ok(conn) = conn_arc.lock() {
            let _ = conn.execute(
                "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_token', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
                [&token],
            );
            let _ = conn.execute(
                "INSERT INTO settings (id, key, value) VALUES (lower(hex(randomblob(16))), 'api_key', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
                [&token],
            );
            let u_id = "online-user".to_string();
            let _ = conn.execute(
                "INSERT INTO user_tokens (id, user_id, token, expires_at, created_at)
                 VALUES (lower(hex(randomblob(16))), ?1, ?2, datetime('now', '+30 days'), datetime('now'));",
                [&u_id, &token],
            );
            if !nama.is_empty() {
                let default_role = "admin".to_string();
                let role_to_save = if role.is_empty() { &default_role } else { &role };
                let _ = conn.execute(
                    "INSERT INTO users (id, username, password_hash, nama, role, is_active, created_at)
                     VALUES (?1, ?2, 'oauth', ?3, ?4, 1, datetime('now'))
                     ON CONFLICT(id) DO UPDATE SET nama = excluded.nama, role = excluded.role;",
                    [&u_id, &nama, &nama, role_to_save],
                );
            }
        }
        {
            let mut st = state.lock().unwrap();
            st.user_token = Some(token.clone());
            if !nama.is_empty() { st.user_nama = nama.clone(); }
            if !role.is_empty() { st.user_role = role.clone(); }
            st.is_logged_in = true;
        }
        if !nama.is_empty() { app.set_user_name(nama.clone().into()); }
        if !role.is_empty() { app.set_user_role(role.into()); }
        app.set_is_logged_in(true);
        app.set_sync_toast_message(format!("Otorisasi web berhasil! Selamat datang, {}.", nama).into());
        app.set_sync_toast_is_error(false);
        refresh_sync_status_ui(app, conn_arc);
    } else if !code.is_empty() {
        let app_weak = app.as_weak();
        let conn_arc_cl = Arc::clone(conn_arc);
        let state_cl = Arc::clone(state);
        std::thread::spawn(move || {
            let api_url: String = {
                if let Ok(conn) = conn_arc_cl.lock() {
                    conn.query_row("SELECT value FROM settings WHERE key = 'api_url';", [], |r| r.get(0)).unwrap_or_default()
                } else {
                    String::new()
                }
            };
            if api_url.is_empty() { return; }
            let rt = tokio::runtime::Runtime::new();
            if let Ok(rt) = rt {
                if let Ok(exchange) = rt.block_on(crate::sync::exchange_code_with_server(&api_url, "", &code)) {
                    if let Ok(conn) = conn_arc_cl.lock() {
                        let _ = crate::commands::save_exchanged_session(&conn, exchange.clone());
                    }
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            {
                                let mut st = state_cl.lock().unwrap();
                                st.user_token = Some(exchange.token.clone());
                                st.user_nama = exchange.user.nama.clone();
                                st.user_role = exchange.user.role.clone();
                                st.is_logged_in = true;
                            }
                            app.set_user_name(exchange.user.nama.into());
                            app.set_user_role(exchange.user.role.into());
                            app.set_is_logged_in(true);
                            app.set_sync_toast_message("Otorisasi kode peramban berhasil!".into());
                            app.set_sync_toast_is_error(false);
                            refresh_sync_status_ui(&app, &conn_arc_cl);
                        }
                    });
                }
            }
        });
    }
}
