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
                                app.set_update_status_text("Aplikasi menggunakan versi terbaru (v0.2.0).".into());
                            }
                        });
                    }
                }
            });
        }
    });
}

fn get_ipc_socket_path() -> std::path::PathBuf {
    let base_dir = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    let app_dir = base_dir.join("me.ppti.partisipasi-sekolah");
    let _ = std::fs::create_dir_all(&app_dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&app_dir, std::fs::Permissions::from_mode(0o700));
    }
    app_dir.join("ipc.sock")
}

pub fn run_slint_app() -> Result<(), Box<dyn std::error::Error>> {
    println!("[INFO] Menjalankan Partisipasi Sekolah Desktop v0.2.0...");

    // Deteksi argumen baris perintah untuk deep-linking (partisipasi-sekolah://)
    let args: Vec<String> = std::env::args().collect();
    let maybe_deep_link = args.iter().find(|a| a.starts_with("partisipasi-sekolah://") || a.starts_with("partisipasi://")).cloned();

    // Pastikan single-instance: jika aplikasi sudah berjalan, alihkan fokus/deep-link lalu keluar
    #[cfg(unix)]
    {
        let socket_path = get_ipc_socket_path();
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&socket_path) {
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
                .unwrap_or_else(|_| "https://sumbangan.smage.my.id".to_string());
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
                    .unwrap_or_else(|_| "https://sumbangan.smage.my.id".to_string())
            } else {
                "https://sumbangan.smage.my.id".to_string()
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
        let socket_path = get_ipc_socket_path();
        let _ = std::fs::remove_file(&socket_path);
        if let Ok(listener) = std::os::unix::net::UnixListener::bind(&socket_path) {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));
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

    // Event listeners & callbacks
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
                            if item.id == aid_str && !item.is_group_header && !item.is_section_header && item.remaining_raw > 0 {
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

                let academic_year_id = state_lock.active_year_id.clone();
                let cashier_nama = state_lock.user_nama.clone();
                let cashier_notes = app.get_cashier_notes().trim().to_string();

                // Generate kode transaksi standar: TRX-YYYYMMDD-XXXXXX (18 Karakter, Paritas 1:1 Web)
                let now = Local::now();
                let random_hex = format!("{:06X}", (Uuid::new_v4().as_u128() % 16777215));
                let tx_code = format!("TRX-{}-{}", now.format("%Y%m%d"), random_hex);
                let tx_id = Uuid::new_v4().to_string();
                let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

                let selected_method = state_lock.selected_method;
                drop(state_lock);

                // Eksekusi Simpan ke SQLite
                let mut success = false;
                if let Ok(mut conn) = conn_arc.lock() {
                    let tx = conn.transaction();
                    if let Ok(tx) = tx {
                        let user_id = get_active_user_id(&tx);

                        // Ambil UUID riil payment_method_id dari tabel payment_methods
                        let method_id: Option<String> = match selected_method {
                            1 => tx.query_row(
                                "SELECT id FROM payment_methods WHERE type = 'bank_transfer' OR name LIKE '%Transfer%' LIMIT 1;",
                                [],
                                |r| r.get(0)
                            ).ok(),
                            2 => tx.query_row(
                                "SELECT id FROM payment_methods WHERE type = 'tabungan' OR name LIKE '%Tabungan%' LIMIT 1;",
                                [],
                                |r| r.get(0)
                            ).ok(),
                            _ => tx.query_row(
                                "SELECT id FROM payment_methods WHERE type = 'cash' OR name LIKE '%Tunai%' LIMIT 1;",
                                [],
                                |r| r.get(0)
                            ).ok(),
                        };

                        // Insert ke payments sesuai schema SQLite & sync engine
                        let res_p = tx.execute(
                            "INSERT INTO payments (id, transaction_code, student_id, user_id, total_amount, payment_method_id, payment_method_name, notes, academic_year_id, created_at, sync_status)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending_push');",
                            params![tx_id, tx_code, student_id, user_id, total_amount, method_id, method_name, cashier_notes, academic_year_id, now_str]
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
                                // Status enum selaras web: 'lunas' atau 'cicilan' (bukan 'sebagian')
                                // sync_status dipertahankan 'synced' agar tidak mengunci pembaruan pull dari server
                                let _ = tx.execute(
                                    "UPDATE payment_assignments 
                                     SET paid_amount = paid_amount + ?1,
                                         status = CASE 
                                             WHEN (paid_amount + ?1 + relief_amount) >= amount THEN 'lunas'
                                             ELSE 'cicilan'
                                         END,
                                         sync_status = 'synced'
                                     WHERE id = ?2;",
                                    params![item.pay_amount, item.assignment_id]
                                );
                            }

                            // Catat pembukuan jurnal umum (Kas Masuk) agar sinkron dengan Laporan Kas Web
                            let student_nama: String = tx.query_row(
                                "SELECT nama FROM students WHERE id = ?1;",
                                [&student_id],
                                |r| r.get(0)
                            ).unwrap_or_else(|_| "Siswa".into());

                            let journal_id = Uuid::new_v4().to_string();
                            let journal_desc = format!("Pembayaran dari {} ({})", student_nama, tx_code);
                            let _ = tx.execute(
                                "INSERT INTO journals (id, academic_year_id, date, type, category, description, amount, reference_id, user_id, created_at, sync_status)
                                 VALUES (?1, ?2, ?3, 'pemasukan', 'Pembayaran Siswa', ?4, ?5, ?6, ?7, ?8, 'pending_push');",
                                params![
                                    journal_id,
                                    academic_year_id,
                                    now.format("%Y-%m-%d").to_string(),
                                    journal_desc,
                                    total_amount,
                                    tx_id,
                                    user_id,
                                    now_str
                                ]
                            );

                            // Jika debet dari Tabungan Siswa
                            if selected_method == 2 {
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
                    app.set_cashier_notes("".into());
                    state.lock().unwrap().cart.clear();
                    // Refresh data siswa & ringkasan
                    load_student_details_ui(&app, &conn_arc, &state, &student_id, false);
                    refresh_today_payments_ui(&app, &conn_arc);
                    refresh_sync_status_ui(&app, &conn_arc);

                    // Buka modal kwitansi otomatis di dalam aplikasi (Tanpa keluar ke peramban)
                    if let Ok(conn) = conn_arc.lock() {
                        let _ = populate_receipt_modal(&app, &conn, &tx_code, &cashier_nama);
                    }
                    show_toast(&app, &format!("Pembayaran {} berhasil!", tx_code), false);
                }
            }
        });
    }

    // H. Trigger Cloud Sync & Full Sync
    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_trigger_sync(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_is_syncing(true);
                app.set_sync_toast_message("Sedang menghubungkan ke server cloud...".into());
                app.set_sync_toast_is_error(false);
                app.set_sync_message("Sedang menghubungkan ke server cloud...".into());

                let app_weak2 = app_weak.clone();
                let conn_arc2 = Arc::clone(&conn_arc);
                let state2 = Arc::clone(&state);

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

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak2.upgrade() {
                            app.set_is_syncing(false);
                            match &result_msg {
                                Ok(msg) => {
                                    app.set_sync_message(msg.clone().into());
                                    show_toast(&app, "Sinkronisasi Berhasil! Data lokal telah diperbarui.", false);
                                    app.set_is_online(true);
                                    println!("[INFO] Sinkronisasi Berhasil: {}", msg);
                                }
                                Err(err) => {
                                    app.set_sync_message(format!("Sync offline: {}", err).into());
                                    show_toast(&app, &format!("Gagal Sinkron: {}", err), true);
                                    println!("[WARN] Sinkronisasi Gagal: {}", err);
                                }
                            }
                            if let Ok(conn) = conn_arc2.lock() {
                                update_branding_and_settings(&app, &conn);
                            }
                            refresh_sync_status_ui(&app, &conn_arc2);
                            refresh_today_payments_ui(&app, &conn_arc2);
                            refresh_history_tab_ui(&app, &conn_arc2, "");

                            // Muat ulang rincian tagihan siswa jika sedang dibuka di layar kasir POS
                            let sid = state2.lock().unwrap().selected_student_id.clone();
                            if let Some(student_id) = sid {
                                load_student_details_ui(&app, &conn_arc2, &state2, &student_id, false);
                            }
                        }
                    });
                });
            }
        });
    }

    {
        let conn_arc = Arc::clone(&conn_arc);
        let state = Arc::clone(&state);
        let app_weak = app_weak.clone();
        app.on_trigger_full_sync(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_is_syncing(true);
                app.set_sync_toast_message("Sedang mengunduh ulang seluruh data dari server pusat...".into());
                app.set_sync_toast_is_error(false);
                app.set_sync_message("Sedang mengunduh ulang seluruh data dari server pusat...".into());

                let app_weak2 = app_weak.clone();
                let conn_arc2 = Arc::clone(&conn_arc);
                let state2 = Arc::clone(&state);

                std::thread::spawn(move || {
                    let db_state = crate::db::DbState {
                        conn: Mutex::new(crate::db::init_db_standalone().unwrap()),
                    };

                    let rt = tokio::runtime::Runtime::new();
                    let result_msg = if let Ok(rt) = rt {
                        rt.block_on(async {
                            crate::commands::perform_full_sync(&db_state).await
                        })
                    } else {
                        Err("Gagal membuat runtime async".into())
                    };

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak2.upgrade() {
                            app.set_is_syncing(false);
                            match &result_msg {
                                Ok(msg) => {
                                    app.set_sync_message(msg.clone().into());
                                    show_toast(&app, "Sinkronisasi Penuh Berhasil! Seluruh data lokal telah selaras.", false);
                                    app.set_is_online(true);
                                    println!("[INFO] Sinkronisasi Penuh Berhasil: {}", msg);
                                }
                                Err(err) => {
                                    app.set_sync_message(format!("Sinkronisasi penuh gagal: {}", err).into());
                                    show_toast(&app, &format!("Gagal Sinkronisasi Penuh: {}", err), true);
                                    println!("[WARN] Sinkronisasi Penuh Gagal: {}", err);
                                }
                            }
                            if let Ok(conn) = conn_arc2.lock() {
                                update_branding_and_settings(&app, &conn);
                            }
                            refresh_sync_status_ui(&app, &conn_arc2);
                            refresh_today_payments_ui(&app, &conn_arc2);
                            refresh_history_tab_ui(&app, &conn_arc2, "");

                            let sid = state2.lock().unwrap().selected_student_id.clone();
                            if let Some(student_id) = sid {
                                load_student_details_ui(&app, &conn_arc2, &state2, &student_id, false);
                            }
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
                            show_toast(&app, "Kwitansi siap ditampilkan.", false);
                        }
                        Err(e) => {
                            show_toast(&app, &format!("Gagal memuat kwitansi: {}", e), true);
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
                            show_toast(&app, "Membuka dialog cetak kuitansi...", false);
                        }
                        Err(e) => {
                            show_toast(&app, &format!("Info Cetak: {}", e), true);
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
                raw_url = format!("https://{}", raw_url);
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
                raw_url = format!("https://{}", raw_url);
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
                show_toast(&app, "Pengaturan sambungan peladen berhasil disimpan.", false);
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
                    show_toast(&app, "URL server tidak boleh kosong.", true);
                }
                return;
            }
            if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
                raw_url = format!("https://{}", raw_url);
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
                                show_toast(&app, "Server terhubung dengan baik.", false);
                                app.set_is_online(true);
                            }
                            Err(e) => {
                                app.set_test_connection_result(format!("Koneksi gagal: {}", e).into());
                                show_toast(&app, &format!("Koneksi gagal: {}", e), true);
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
                show_toast(&app, msg, false);
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
                raw_url = format!("https://{}", raw_url);
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
                                app.set_wizard_test_message(format!("Berhasil: {}", msg).into());
                                app.set_wizard_test_success(true);
                            }
                            Err(e) => {
                                app.set_wizard_test_message(format!("Sambungan gagal: {}", e).into());
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
                raw_url = format!("https://{}", raw_url);
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
                show_toast(&app, "Konfigurasi awal berhasil diselesaikan!", false);
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
        let state_sync = Arc::clone(&state);
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
                    "https://sumbangan.smage.my.id".to_string()
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
                            let state_res = Arc::clone(&state_sync);
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
                                            refresh_history_tab_ui(&app, &conn_arc_res, "");

                                            let sid = state_res.lock().unwrap().selected_student_id.clone();
                                            if let Some(student_id) = sid {
                                                load_student_details_ui(&app, &conn_arc_res, &state_res, &student_id, false);
                                            }
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

    // Jalankan Event Loop Antarmuka Slint UI
    app.run()?;
    Ok(())
}

pub fn run_slint_native() -> Result<(), Box<dyn std::error::Error>> {
    run_slint_app()
}

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

fn load_and_cache_stamp_image(conn: &Connection) -> Option<std::path::PathBuf> {
    let stamp_data: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'stamp_image' AND value != '' LIMIT 1;",
        [],
        |r| r.get(0)
    ).unwrap_or_default();

    let cache_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("me.ppti.partisipasi-sekolah");
    let _ = std::fs::create_dir_all(&cache_dir);
    let stamp_file = cache_dir.join("cached_stamp_image.png");

    if !stamp_data.is_empty() {
        if let Some(pos) = stamp_data.find("base64,") {
            let b64_str = &stamp_data[pos + 7..];
            if let Some(bytes) = base64_decode(b64_str) {
                let _ = std::fs::write(&stamp_file, &bytes);
                return Some(stamp_file);
            }
        }
    }

    if stamp_file.exists() {
        Some(stamp_file)
    } else {
        None
    }
}

fn format_indo_date(date_str: &str) -> String {
    let date_part = date_str.split(' ').next().unwrap_or(date_str);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() == 3 {
        let day = parts[2];
        let month = match parts[1] {
            "01" => "Januari",
            "02" => "Februari",
            "03" => "Maret",
            "04" => "April",
            "05" => "Mei",
            "06" => "Juni",
            "07" => "Juli",
            "08" => "Agustus",
            "09" => "September",
            "10" => "Oktober",
            "11" => "November",
            "12" => "Desember",
            _ => parts[1],
        };
        let year = parts[0];
        format!("{} {} {}", day, month, year)
    } else {
        date_str.to_string()
    }
}

fn show_toast(app: &AppWindow, message: &str, is_error: bool) {
    let msg = message.to_string();
    app.set_sync_toast_message(msg.clone().into());
    app.set_sync_toast_is_error(is_error);

    let app_weak = app.as_weak();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(6));
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak.upgrade() {
                if app.get_sync_toast_message().to_string() == msg {
                    app.set_sync_toast_message("".into());
                }
            }
        });
    });
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
         WHERE d.payment_id = ?1
         ORDER BY COALESCE(y.name, '') ASC, (CASE WHEN a.month IS NULL THEN 999 WHEN a.month >= 7 THEN a.month - 6 ELSE a.month + 6 END) ASC, t.name ASC;"
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

    // Paritas 1:1 Web: Suffix T.A hanya ditampilkan jika transaksi mencakup lebih dari 1 tahun ajaran
    let unique_years: std::collections::HashSet<String> = items_raw.iter()
        .filter_map(|item| item.3.clone())
        .collect();
    let show_year_suffix = unique_years.len() > 1;

    let show_nominal_setting: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'show_nominal_receipt';",
        [],
        |r| r.get(0)
    ).unwrap_or_else(|_| "1".into());
    let show_nominal = show_nominal_setting != "0";

    let mut modal_items = Vec::new();
    for (i, item) in items_raw.iter().enumerate() {
        let mut desc = item.0.clone();
        if let Some(m) = item.1 {
            let m_name = month_name(m);
            if !m_name.is_empty() {
                desc.push_str(&format!(" ({})", m_name));
            }
        }
        if show_year_suffix {
            if let Some(ref y) = item.3 {
                desc.push_str(&format!(" (T.A {})", y));
            }
        }
        let amt_str = if show_nominal {
            format_idr(item.2)
        } else {
            "Sudah Lunas".to_string()
        };
        modal_items.push(ReceiptModalItem {
            index: (i + 1) as i32,
            description: desc.into(),
            amount: amt_str.into(),
        });
    }

    let school_name: String = conn.query_row("SELECT value FROM settings WHERE key = 'school_name';", [], |r| r.get(0)).unwrap_or_else(|_| "Nama Sekolah".into());
    let school_address: String = conn.query_row("SELECT value FROM settings WHERE key = 'school_address';", [], |r| r.get(0)).unwrap_or_else(|_| "Alamat Sekolah".into());
    let committee_name: String = conn.query_row("SELECT value FROM settings WHERE key = 'committee_name';", [], |r| r.get(0)).unwrap_or_default();
    let receipt_date = format!("Mojokerto, {}", format_indo_date(&created_at));

    let mut has_stamp = false;
    if let Some(stamp_path) = load_and_cache_stamp_image(conn) {
        if let Ok(img) = slint::Image::load_from_path(&stamp_path) {
            app.set_has_receipt_stamp(true);
            app.set_receipt_stamp_image(img);
            has_stamp = true;
        }
    }
    if !has_stamp {
        app.set_has_receipt_stamp(false);
    }

    let display_total = if show_nominal {
        format_idr(total_amount)
    } else {
        "Sudah Lunas".to_string()
    };

    app.set_receipt_modal_data(ReceiptModalData {
        transaction_code: trx_code.into(),
        created_at: created_at.into(),
        receipt_date: receipt_date.into(),
        student_name: student_nama.into(),
        student_nis: student_nis.unwrap_or_else(|| "-".into()).into(),
        student_class: student_kelas.into(),
        payment_method: payment_method_name.into(),
        total_amount: display_total.into(),
        cashier_name: if cashier_name.is_empty() { "Petugas Kasir".into() } else { cashier_name.into() },
        notes: notes.unwrap_or_default().into(),
        committee_name: committee_name.into(),
        school_name: school_name.into(),
        school_address: school_address.into(),
        has_stamp,
    });
    app.set_receipt_modal_items(Rc::new(VecModel::from(modal_items)).into());
    app.set_show_receipt_modal(true);

    Ok(())
}

fn get_setting_val(conn: &Connection, key: &str, default_val: &str) -> String {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1 LIMIT 1;",
        [key],
        |r| r.get(0),
    ).unwrap_or_else(|_| default_val.to_string())
}

fn get_image_data_uri(conn: &Connection, key: &str) -> Option<String> {
    let raw = get_setting_val(conn, key, "");
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("data:image/") {
        Some(trimmed.to_string())
    } else if trimmed.len() > 60 && !trimmed.starts_with("http") && !trimmed.starts_with('/') {
        Some(format!("data:image/png;base64,{}", trimmed))
    } else {
        None
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#39;")
}

fn generate_receipt_html(conn: &Connection, trx_code: &str, cashier_name: &str) -> Result<String, String> {
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

    let school_name = get_setting_val(conn, "school_name", "Nama Sekolah");
    let school_address = get_setting_val(conn, "school_address", "Alamat Sekolah");
    let committee_name = get_setting_val(conn, "committee_name", "");
    let show_nominal = get_setting_val(conn, "show_nominal_receipt", "1") != "0";
    let show_nominal_bebas = get_setting_val(conn, "show_nominal_receipt_bebas", "1") != "0";

    let logo_left = get_image_data_uri(conn, "logo_left")
        .or_else(|| get_image_data_uri(conn, "app_logo"))
        .or_else(|| {
            load_and_cache_school_logo(conn).and_then(|p| {
                std::fs::read(&p).ok().map(|b| format!("data:image/png;base64,{}", crate::commands::base64_encode(&b)))
            })
        });

    let logo_right = get_image_data_uri(conn, "logo_right");

    let stamp_img = get_image_data_uri(conn, "stamp_image")
        .or_else(|| {
            load_and_cache_stamp_image(conn).and_then(|p| {
                std::fs::read(&p).ok().map(|b| format!("data:image/png;base64,{}", crate::commands::base64_encode(&b)))
            })
        });

    let mut stmt = conn.prepare(
        "SELECT t.name, t.type, a.month, d.amount, y.name
         FROM payment_details d
         JOIN payment_assignments a ON d.payment_assignment_id = a.id
         JOIN payment_types t ON a.payment_type_id = t.id
         LEFT JOIN academic_years y ON a.academic_year_id = y.id
         WHERE d.payment_id = ?1
         ORDER BY COALESCE(y.name, '') ASC, (CASE WHEN a.month IS NULL THEN 999 WHEN a.month >= 7 THEN a.month - 6 ELSE a.month + 6 END) ASC, t.name ASC;"
    ).map_err(|e| e.to_string())?;

    let month_name = |m: i32| -> &'static str {
        match m {
            7 => "Juli", 8 => "Agustus", 9 => "September", 10 => "Oktober",
            11 => "November", 12 => "Desember", 1 => "Januari", 2 => "Februari",
            3 => "Maret", 4 => "April", 5 => "Mei", 6 => "Juni", _ => "",
        }
    };

    let items_raw: Vec<(String, String, Option<i32>, f64, Option<String>)> = stmt.query_map([&payment_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i32>>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    }).map_err(|e| e.to_string())?.flatten().collect();

    let unique_years: std::collections::HashSet<String> = items_raw.iter()
        .filter_map(|item| item.4.clone())
        .collect();
    let show_year_suffix = unique_years.len() > 1;

    let mut items_rows_html = String::new();
    let mut visible_nominal_total = 0.0;

    for (i, item) in items_raw.iter().enumerate() {
        let is_bebas = item.1 == "bebas";
        let show_amount_for_row = show_nominal || (is_bebas && show_nominal_bebas);
        if show_amount_for_row {
            visible_nominal_total += item.3;
        }

        let mut desc = item.0.clone();
        if let Some(m) = item.2 {
            let mn = month_name(m);
            if !mn.is_empty() {
                desc.push_str(&format!(" ({})", mn));
            }
        }
        if show_year_suffix {
            if let Some(ref y) = item.4 {
                desc.push_str(&format!(" (T.A {})", y));
            }
        }

        let amount_cell = if show_amount_for_row {
            format!("<td class=\"amount\">{}</td>", format_idr(item.3))
        } else {
            "<td><span class=\"badge-lunas\">Sudah Lunas</span></td>".to_string()
        };

        items_rows_html.push_str(&format!(
            "<tr><td style=\"text-align: center;\">{}</td><td>{}</td>{}</tr>\n",
            i + 1,
            html_escape(&desc),
            amount_cell
        ));
    }

    let tfoot_html = if show_nominal || visible_nominal_total > 0.0 {
        let grand_total = if show_nominal { total_amount } else { visible_nominal_total };
        format!(
            "<tfoot><tr><td colspan=\"2\" style=\"text-align: right; font-weight: 700;\">Total Pembayaran</td><td class=\"amount\">{}</td></tr></tfoot>",
            format_idr(grand_total)
        )
    } else {
        String::new()
    };

    let amount_header = if show_nominal {
        "Jumlah"
    } else {
        "Jumlah / Status"
    };

    let logo_left_html = if let Some(ref l) = logo_left {
        format!("<img src=\"{}\" alt=\"Logo\" class=\"receipt-logo\">", l)
    } else {
        String::new()
    };

    let logo_right_html = if let Some(ref r) = logo_right {
        format!("<img src=\"{}\" alt=\"Logo\" class=\"receipt-logo\">", r)
    } else {
        String::new()
    };

    let committee_html = if !committee_name.trim().is_empty() {
        format!("<h2>{}</h2>", html_escape(committee_name.trim()))
    } else {
        String::new()
    };

    let notes_html = if let Some(ref n) = notes {
        let n_trimmed = n.trim();
        if !n_trimmed.is_empty() {
            format!("<div class=\"notes-section\"><strong>Catatan:</strong> {}</div>", html_escape(n_trimmed))
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let stamp_html = if let Some(ref s) = stamp_img {
        format!("<img src=\"{}\" alt=\"Stempel\" class=\"receipt-stamp-img\">", s)
    } else {
        String::new()
    };

    let receipt_date = format_indo_date(&created_at);
    let cashier_display = if cashier_name.trim().is_empty() { "Petugas Kasir" } else { cashier_name.trim() };
    let now_str = chrono::Local::now().format("%d/%m/%Y %H:%M").to_string();

    let html = format!(r##"<!DOCTYPE html>
<html lang="id">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kwitansi Pembayaran - {safe_trx_code}</title>
    <style>
        @page {{
            size: A4 portrait;
            margin: 15mm;
        }}
        * {{
            box-sizing: border-box;
            -webkit-print-color-adjust: exact !important;
            print-color-adjust: exact !important;
        }}
        body {{
            background-color: #f1f5f9;
            color: #0f172a;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            margin: 0;
            padding: 24px;
        }}
        .no-print-toolbar {{
            max-width: 800px;
            margin: 0 auto 16px auto;
            display: flex;
            justify-content: space-between;
            align-items: center;
            background: #002b59;
            color: #ffffff;
            padding: 12px 20px;
            border-radius: 8px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
        }}
        .no-print-toolbar .title-info strong {{
            display: block;
            font-size: 14px;
            font-weight: 700;
        }}
        .no-print-toolbar .title-info span {{
            font-size: 11px;
            color: #cbd5e1;
        }}
        .no-print-toolbar .buttons {{
            display: flex;
            gap: 10px;
        }}
        .btn {{
            cursor: pointer;
            border: none;
            padding: 8px 18px;
            font-size: 12px;
            font-weight: 700;
            border-radius: 6px;
            transition: all 0.15s ease;
        }}
        .btn-print {{
            background: #008f5d;
            color: #ffffff;
        }}
        .btn-print:hover {{
            background: #006b45;
        }}
        .btn-close {{
            background: #e2e8f0;
            color: #1e293b;
        }}
        .btn-close:hover {{
            background: #cbd5e1;
        }}
        .receipt {{
            max-width: 800px;
            margin: 0 auto;
            padding: 36px 40px;
            background: #ffffff;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.08);
            border: 1px solid #e2e8f0;
        }}
        .receipt-header {{
            display: flex;
            align-items: center;
            gap: 20px;
            padding-bottom: 14px;
            border-bottom: 3px double #002b59;
            margin-bottom: 22px;
        }}
        .receipt-logo {{
            width: 72px;
            height: 72px;
            object-fit: contain;
        }}
        .receipt-school {{
            flex: 1;
            text-align: center;
        }}
        .receipt-school h2 {{
            font-size: 1.15rem;
            margin: 0 0 4px 0;
            color: #002b59;
            font-weight: 800;
            letter-spacing: 0.02em;
        }}
        .receipt-school p {{
            font-size: 0.78rem;
            margin: 0;
            color: #475569;
            line-height: 1.35;
        }}
        .receipt-title {{
            text-align: center;
            font-size: 1.05rem;
            font-weight: 800;
            text-transform: uppercase;
            letter-spacing: 0.08em;
            margin-bottom: 20px;
            color: #002b59;
        }}
        .receipt-table {{
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 20px;
        }}
        .receipt-table td {{
            padding: 4px 6px;
            font-size: 0.85rem;
            vertical-align: top;
        }}
        .receipt-table .label {{
            width: 120px;
            font-weight: 600;
            color: #475569;
        }}
        .receipt-items {{
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 20px;
        }}
        .receipt-items th,
        .receipt-items td {{
            border: 1px solid #cbd5e1;
            padding: 8px 12px;
            font-size: 0.82rem;
        }}
        .receipt-items th {{
            background: #f8fafc;
            font-weight: 700;
            text-align: left;
            color: #0f172a;
        }}
        .receipt-items .amount {{
            text-align: right;
            font-variant-numeric: tabular-nums;
            font-weight: 600;
        }}
        .receipt-items tfoot td {{
            font-weight: 700;
            border-top: 2px solid #002b59;
            background: #fafafa;
        }}
        .badge-lunas {{
            display: inline-block;
            background: #dcfce7;
            color: #15803d;
            font-size: 11px;
            font-weight: 700;
            padding: 2px 8px;
            border-radius: 4px;
            border: 1px solid #bbf7d0;
        }}
        .notes-section {{
            font-size: 0.82rem;
            margin-bottom: 20px;
            color: #334155;
            padding: 8px 14px;
            background: #f8fafc;
            border-radius: 6px;
            border-left: 3px solid #002b59;
        }}
        .receipt-footer {{
            display: flex;
            justify-content: flex-end;
            margin-top: 24px;
        }}
        .receipt-sign-container {{
            position: relative;
            min-height: 130px;
            width: 240px;
            text-align: center;
        }}
        .receipt-stamp-img {{
            position: absolute;
            left: 10px;
            top: 15px;
            width: 125px;
            object-fit: contain;
            opacity: 0.82;
            z-index: 1;
            transform: rotate(-10deg);
            pointer-events: none;
        }}
        .receipt-sign-text {{
            position: relative;
            z-index: 2;
            font-size: 0.82rem;
            line-height: 1.4;
        }}
        .receipt-sign-cashier {{
            position: absolute;
            left: 0;
            right: 0;
            bottom: 0;
            z-index: 2;
            border-top: 1px solid #0f172a;
            padding-top: 4px;
            font-size: 0.85rem;
            font-weight: 700;
        }}
        .receipt-timestamp {{
            text-align: center;
            font-size: 0.68rem;
            color: #94a3b8;
            margin-top: 32px;
        }}

        @media print {{
            body {{
                background: white !important;
                padding: 0 !important;
            }}
            .no-print {{
                display: none !important;
            }}
            .receipt {{
                max-width: 100% !important;
                margin: 0 !important;
                padding: 0 !important;
                box-shadow: none !important;
                border: none !important;
                border-radius: 0 !important;
            }}
        }}
    </style>
</head>
<body>
    <div class="no-print no-print-toolbar">
        <div class="title-info">
            <strong>Pratinjau Kuitansi Pembayaran</strong>
            <span>Siap dicetak menggunakan printer standar / printer biasa (A4).</span>
        </div>
        <div class="buttons">
            <button class="btn btn-print" onclick="window.print()">Cetak (Buka Dialog Print)</button>
            <button class="btn btn-close" onclick="window.close()">Tutup</button>
        </div>
    </div>

    <div class="receipt">
        <div class="receipt-header">
            {logo_left_html}
            <div class="receipt-school">
                {committee_html}
                <h2>{school_name}</h2>
                <p>{school_address}</p>
            </div>
            {logo_right_html}
        </div>

        <div class="receipt-title">Kwitansi Pembayaran</div>

        <table class="receipt-table">
            <tr>
                <td class="label">No. Transaksi</td>
                <td>: <strong>{trx_code}</strong></td>
                <td class="label">Tanggal</td>
                <td>: {created_at}</td>
            </tr>
            <tr>
                <td class="label">Nama Siswa</td>
                <td>: <strong>{student_nama}</strong></td>
                <td class="label">NIS</td>
                <td>: {student_nis}</td>
            </tr>
            <tr>
                <td class="label">Kelas</td>
                <td>: {student_kelas}</td>
                <td class="label">Metode Bayar</td>
                <td>: <strong>{payment_method_name}</strong></td>
            </tr>
        </table>

        <table class="receipt-items">
            <thead>
                <tr>
                    <th style="width: 40px; text-align: center;">No</th>
                    <th>Keterangan</th>
                    <th style="width: 170px;" class="amount">{amount_header}</th>
                </tr>
            </thead>
            <tbody>
                {items_rows_html}
            </tbody>
            {tfoot_html}
        </table>

        {notes_html}

        <div class="receipt-footer">
            <div class="receipt-sign-container">
                {stamp_html}
                <div class="receipt-sign-text">
                    <p style="margin: 0;">Mojokerto, {receipt_date}</p>
                    <p style="margin: 0; font-weight: 700;">Petugas Penerima,</p>
                </div>
                <div class="receipt-sign-cashier">
                    {cashier_display}
                </div>
            </div>
        </div>

        <p class="receipt-timestamp">
            Dicetak pada {now_str} • {school_name}
        </p>
    </div>

    <script>
        window.addEventListener('load', function() {{
            setTimeout(function() {{
                window.print();
            }}, 250);
        }});
    </script>
</body>
</html>
"##,
        safe_trx_code = html_escape(trx_code),
        logo_left_html = logo_left_html,
        committee_html = committee_html,
        school_name = html_escape(&school_name),
        school_address = html_escape(&school_address),
        logo_right_html = logo_right_html,
        trx_code = html_escape(trx_code),
        created_at = html_escape(&created_at),
        student_nama = html_escape(&student_nama),
        student_nis = html_escape(&student_nis.unwrap_or_else(|| "-".to_string())),
        student_kelas = html_escape(&student_kelas),
        payment_method_name = html_escape(&payment_method_name),
        amount_header = amount_header,
        items_rows_html = items_rows_html,
        tfoot_html = tfoot_html,
        notes_html = notes_html,
        stamp_html = stamp_html,
        receipt_date = html_escape(&receipt_date),
        cashier_display = html_escape(cashier_display),
        now_str = html_escape(&now_str)
    );

    Ok(html)
}

fn send_receipt_to_printer(conn: &Connection, trx_code: &str, cashier_name: &str) -> Result<(), String> {
    let html_content = generate_receipt_html(conn, trx_code, cashier_name)?;

    let safe_trx_code: String = trx_code.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    let tmp_path = std::env::temp_dir().join(format!("kwitansi_{}.html", safe_trx_code));
    std::fs::write(&tmp_path, html_content)
        .map_err(|e| format!("Gagal membuat berkas kuitansi cetak: {}", e))?;

    // Buka berkas HTML menggunakan peramban/penampil default sistem untuk memicu dialog print asli
    #[cfg(windows)]
    {
        let opened = open::that(&tmp_path);
        if opened.is_err() {
            let rundll = std::process::Command::new("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", &tmp_path.display().to_string()])
                .spawn();
            if rundll.is_err() {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &tmp_path.display().to_string()])
                    .spawn();
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if open::that(&tmp_path).is_err() {
            let _ = std::process::Command::new("open").arg(&tmp_path).spawn();
        }
    }

    #[cfg(target_os = "linux")]
    {
        if open::that(&tmp_path).is_err() {
            let _ = std::process::Command::new("xdg-open").arg(&tmp_path).spawn();
        }
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
            "SELECT 
                a.id, 
                t.name, 
                t.type, 
                a.month, 
                a.amount, 
                a.paid_amount, 
                a.relief_amount, 
                a.status, 
                COALESCE(y.name, 'Lainnya') as academic_year_name, 
                COALESCE(y.is_active, 0) as is_active,
                pd.last_payment_date,
                COALESCE(inst.inst_count, 0) as inst_count
             FROM payment_assignments a
             JOIN payment_types t ON a.payment_type_id = t.id
             LEFT JOIN academic_years y ON a.academic_year_id = y.id
             LEFT JOIN (
                 SELECT payment_assignment_id, MAX(created_at) as last_payment_date
                 FROM payment_details
                 GROUP BY payment_assignment_id
             ) pd ON a.id = pd.payment_assignment_id
             LEFT JOIN (
                 SELECT payment_assignment_id, COUNT(*) as inst_count
                 FROM payment_details
                 GROUP BY payment_assignment_id
             ) inst ON a.id = inst.payment_assignment_id
             WHERE a.student_id = ?1
             ORDER BY 
                 COALESCE(y.name, '0000') DESC, 
                 t.type ASC, 
                 (CASE WHEN a.month IS NULL THEN 999 WHEN a.month >= 7 THEN a.month - 6 ELSE a.month + 6 END) ASC, 
                 t.name ASC;"
        ) {
            struct ParsedBill {
                id: String,
                name: String,
                t_type: String,
                month_name: String,
                amount: f64,
                paid: f64,
                relief: f64,
                remaining: f64,
                status_key: String,
                status_label: String,
                year_name: String,
                is_active: bool,
                last_payment_date: String,
                installments_text: String,
            }

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
                let last_payment_date: Option<String> = row.get(10).unwrap_or(None);
                let inst_count: i32 = row.get(11).unwrap_or(0);

                let month_name = match month {
                    Some(7) => "Juli", Some(8) => "Agustus", Some(9) => "September",
                    Some(10) => "Oktober", Some(11) => "November", Some(12) => "Desember",
                    Some(1) => "Januari", Some(2) => "Februari", Some(3) => "Maret",
                    Some(4) => "April", Some(5) => "Mei", Some(6) => "Juni",
                    _ => "",
                };

                let remaining = (amount - paid - relief).max(0.0);
                let is_lunas = remaining <= 0.0 || status == "lunas" || status == "bebas";
                let is_cicilan = !is_lunas && (paid > 0.0 || relief > 0.0 || status == "cicilan" || status == "sebagian");

                let status_key = if is_lunas {
                    "lunas".to_string()
                } else if is_cicilan {
                    "cicilan".to_string()
                } else {
                    "belum_bayar".to_string()
                };

                let last_date_str = last_payment_date.clone().unwrap_or_default();
                let status_label = if is_lunas {
                    if let Some(ref dt) = last_payment_date {
                        let clean_dt = if dt.len() >= 10 {
                            let parts: Vec<&str> = dt[..10].split('-').collect();
                            if parts.len() == 3 {
                                format!("{}/{}/{}", parts[2], parts[1], parts[0])
                            } else {
                                dt[..10].to_string()
                            }
                        } else {
                            dt.clone()
                        };
                        format!("LUNAS: {}", clean_dt)
                    } else {
                        "LUNAS".to_string()
                    }
                } else if is_cicilan {
                    "CICILAN".to_string()
                } else {
                    "BELUM BAYAR".to_string()
                };

                let installments_text = if t_type == "bebas" && inst_count > 0 {
                    format!("Riwayat Cicilan ({}x)", inst_count)
                } else {
                    "".to_string()
                };

                Ok(ParsedBill {
                    id,
                    name,
                    t_type,
                    month_name: month_name.to_string(),
                    amount,
                    paid,
                    relief,
                    remaining,
                    status_key,
                    status_label,
                    year_name,
                    is_active: is_active == 1,
                    last_payment_date: last_date_str,
                    installments_text,
                })
            }) {
                let all_rows: Vec<ParsedBill> = rows.flatten().collect();
                let mut year_stats: std::collections::HashMap<String, (i32, f64)> = std::collections::HashMap::new();

                for r in &all_rows {
                    total_tagihan += r.amount;
                    total_dibayar += r.paid;
                    total_keringanan += r.relief;
                    total_tunggakan += r.remaining;

                    let stat = year_stats.entry(r.year_name.clone()).or_insert((0, 0.0));
                    stat.0 += 1;
                    stat.1 += r.remaining;
                }

                if reset_collapse {
                    let mut st = state.lock().unwrap();
                    st.collapsed_years.clear();
                    for r in &all_rows {
                        if !r.is_active {
                            st.collapsed_years.insert(r.year_name.clone());
                        }
                    }
                }

                // Dapatkan urutan tahun pelajaran
                let mut distinct_years = Vec::new();
                for r in &all_rows {
                    if !distinct_years.iter().any(|(y, _)| y == &r.year_name) {
                        distinct_years.push((r.year_name.clone(), r.is_active));
                    }
                }

                for (year_name, is_active) in distinct_years {
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

                    // Header Tahun Pelajaran
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
                        status_label: "".into(),
                        last_payment_date: "".into(),
                        is_selected: false,
                        is_group_header: true,
                        group_header_title: header_title.into(),
                        is_section_header: false,
                        section_title: "".into(),
                        section_count_text: "".into(),
                        section_subtotal_text: "".into(),
                        is_active_year: is_active,
                        is_collapsed,
                        is_hidden: false,
                        total_bills_in_year: count_bills,
                        total_remaining_in_year: rem_text.into(),
                        installments_text: "".into(),
                    });

                    if !is_collapsed {
                        let year_rows: Vec<&ParsedBill> = all_rows.iter().filter(|r| r.year_name == year_name).collect();
                        let bebas_rows: Vec<&ParsedBill> = year_rows.iter().filter(|r| r.t_type == "bebas").cloned().collect();
                        let bulanan_rows: Vec<&ParsedBill> = year_rows.iter().filter(|r| r.t_type == "bulanan").cloned().collect();

                        // 1. Seksi Tagihan Bebas (Jika ada)
                        if !bebas_rows.is_empty() {
                            let bebas_subtotal: f64 = bebas_rows.iter().map(|r| r.remaining).sum();
                            assignments_vec.push(AssignmentData {
                                id: format!("sec-bebas-{}", year_name).into(),
                                academic_year_name: year_name.clone().into(),
                                payment_type_name: "".into(),
                                payment_type_type: "bebas".into(),
                                month_name: "".into(),
                                amount: "".into(),
                                paid_amount: "".into(),
                                remaining_amount: "".into(),
                                remaining_raw: 0,
                                status: "".into(),
                                status_label: "".into(),
                                last_payment_date: "".into(),
                                is_selected: false,
                                is_group_header: false,
                                group_header_title: "".into(),
                                is_section_header: true,
                                section_title: "Tagihan Bebas".into(),
                                section_count_text: format!("{} ITEM", bebas_rows.len()).into(),
                                section_subtotal_text: format!("Subtotal: {}", format_idr(bebas_subtotal)).into(),
                                is_active_year: is_active,
                                is_collapsed: false,
                                is_hidden: false,
                                total_bills_in_year: 0,
                                total_remaining_in_year: "".into(),
                                installments_text: "".into(),
                            });

                            for r in bebas_rows {
                                let is_sel = state.lock().unwrap().cart.iter().any(|c| c.assignment_id == r.id);
                                assignments_vec.push(AssignmentData {
                                    id: r.id.clone().into(),
                                    academic_year_name: year_name.clone().into(),
                                    payment_type_name: r.name.clone().into(),
                                    payment_type_type: r.t_type.clone().into(),
                                    month_name: r.month_name.clone().into(),
                                    amount: format_idr(r.amount).into(),
                                    paid_amount: format_idr(r.paid).into(),
                                    remaining_amount: format_idr(r.remaining).into(),
                                    remaining_raw: r.remaining as i32,
                                    status: r.status_key.clone().into(),
                                    status_label: r.status_label.clone().into(),
                                    last_payment_date: r.last_payment_date.clone().into(),
                                    is_selected: is_sel,
                                    is_group_header: false,
                                    group_header_title: "".into(),
                                    is_section_header: false,
                                    section_title: "".into(),
                                    section_count_text: "".into(),
                                    section_subtotal_text: "".into(),
                                    is_active_year: is_active,
                                    is_collapsed: false,
                                    is_hidden: false,
                                    total_bills_in_year: 0,
                                    total_remaining_in_year: "".into(),
                                    installments_text: r.installments_text.clone().into(),
                                });
                            }
                        }

                        // 2. Seksi Tagihan Bulanan (Jika ada)
                        if !bulanan_rows.is_empty() {
                            let bulanan_subtotal: f64 = bulanan_rows.iter().map(|r| r.remaining).sum();
                            assignments_vec.push(AssignmentData {
                                id: format!("sec-bulanan-{}", year_name).into(),
                                academic_year_name: year_name.clone().into(),
                                payment_type_name: "".into(),
                                payment_type_type: "bulanan".into(),
                                month_name: "".into(),
                                amount: "".into(),
                                paid_amount: "".into(),
                                remaining_amount: "".into(),
                                remaining_raw: 0,
                                status: "".into(),
                                status_label: "".into(),
                                last_payment_date: "".into(),
                                is_selected: false,
                                is_group_header: false,
                                group_header_title: "".into(),
                                is_section_header: true,
                                section_title: "Tagihan Bulanan".into(),
                                section_count_text: format!("{} ITEM", bulanan_rows.len()).into(),
                                section_subtotal_text: format!("Subtotal: {}", format_idr(bulanan_subtotal)).into(),
                                is_active_year: is_active,
                                is_collapsed: false,
                                is_hidden: false,
                                total_bills_in_year: 0,
                                total_remaining_in_year: "".into(),
                                installments_text: "".into(),
                            });

                            for r in bulanan_rows {
                                let is_sel = state.lock().unwrap().cart.iter().any(|c| c.assignment_id == r.id);
                                assignments_vec.push(AssignmentData {
                                    id: r.id.clone().into(),
                                    academic_year_name: year_name.clone().into(),
                                    payment_type_name: r.name.clone().into(),
                                    payment_type_type: r.t_type.clone().into(),
                                    month_name: r.month_name.clone().into(),
                                    amount: format_idr(r.amount).into(),
                                    paid_amount: format_idr(r.paid).into(),
                                    remaining_amount: format_idr(r.remaining).into(),
                                    remaining_raw: r.remaining as i32,
                                    status: r.status_key.clone().into(),
                                    status_label: r.status_label.clone().into(),
                                    last_payment_date: r.last_payment_date.clone().into(),
                                    is_selected: is_sel,
                                    is_group_header: false,
                                    group_header_title: "".into(),
                                    is_section_header: false,
                                    section_title: "".into(),
                                    section_count_text: "".into(),
                                    section_subtotal_text: "".into(),
                                    is_active_year: is_active,
                                    is_collapsed: false,
                                    is_hidden: false,
                                    total_bills_in_year: 0,
                                    total_remaining_in_year: "".into(),
                                    installments_text: r.installments_text.clone().into(),
                                });
                            }
                        }
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
            if !item.is_group_header && !item.is_section_header {
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
    println!("[INFO] Memproses deep-link otorisasi URL...");
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
        show_toast(app, &format!("Otorisasi web berhasil! Selamat datang, {}.", nama), false);
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
                            show_toast(&app, "Otorisasi kode peramban berhasil!", false);
                            refresh_sync_status_ui(&app, &conn_arc_cl);
                        }
                    });
                }
            }
        });
    }
}
