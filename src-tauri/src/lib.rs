mod db;
mod migrations;
mod sync;
mod commands;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt;

pub struct PendingDeepLink(pub Mutex<Option<String>>);

fn spawn_background_sync(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Beri jeda awal 10 detik agar window dan db terinisialisasi penuh
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        loop {
            if let Some(db_state) = app_handle.try_state::<db::DbState>() {
                // Periksa apakah sesi pengguna aktif
                let has_session = {
                    if let Ok(conn) = db_state.conn.lock() {
                        conn.query_row("SELECT 1 FROM user_tokens LIMIT 1;", [], |_| Ok(true))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                };

                if has_session {
                    match commands::perform_sync(&db_state).await {
                        Ok(msg) => {
                            let _ = app_handle.emit("background-sync-success", msg);
                        }
                        Err(err) => {
                            // Diam saat offline atau peladen tidak terjangkau (failover mode)
                            let _ = app_handle.emit("background-sync-error", err);
                        }
                    }
                }
            }

            // Interval sync berkala setiap 30 detik (menjaga data kasir lokal selalu up-to-date)
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            // 1. Inisialisasi Database SQLite lokal
            let conn = db::init_db(app.handle())
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
                
            // 2. Jalankan Migrasi & Seeder otomatis
            db::run_migrations(&conn)
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
                
            // 3. Daftarkan Koneksi Database ke State Manager Tauri agar bisa diakses di commands
            app.manage(db::DbState {
                conn: Mutex::new(conn),
            });

            // 4. Daftarkan State untuk Deep Link pending (mencegah cold-start race condition)
            app.manage(PendingDeepLink(Mutex::new(None)));

            // 5. Setup Deep Link Listener
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                if let Some(url) = event.urls().first() {
                    let url_str = url.to_string();
                    if let Some(state) = handle.try_state::<PendingDeepLink>() {
                        *state.0.lock().unwrap() = Some(url_str.clone());
                    }
                    let _ = handle.emit("desktop-login-success", url_str);
                }
            });

            // Check if application was launched by clicking a deep link directly on startup
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                if let Some(url) = urls.first() {
                    let url_str = url.to_string();
                    if let Some(state) = app.try_state::<PendingDeepLink>() {
                        *state.0.lock().unwrap() = Some(url_str.clone());
                    }
                    let _ = app.emit("desktop-login-success", url_str);
                }
            }

            // 6. Setup System Tray Icon & Context Menu (mirip Warp Tunnel / Tailscale)
            let show_item = MenuItem::with_id(app, "show", "Buka Partisipasi Sekolah", true, None::<&str>)?;
            let sync_item = MenuItem::with_id(app, "sync_now", "Sinkronkan Sekarang", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Keluar dari Aplikasi", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &sync_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("Partisipasi Sekolah - Failover Sync Engine")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "sync_now" => {
                            let handle_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Some(db_state) = handle_clone.try_state::<db::DbState>() {
                                    if let Ok(msg) = commands::perform_sync(&db_state).await {
                                        let _ = handle_clone.emit("background-sync-success", msg);
                                    }
                                }
                            });
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // 7. Mulai daemon background auto-sync worker
            spawn_background_sync(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Ketika tombol [X] ditekan, sembunyikan ke system tray (tidak langsung exit process)
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_students,
            commands::get_student_details,
            commands::save_payment,
            commands::save_savings_transaction,
            commands::get_sync_config,
            commands::set_sync_config,
            commands::trigger_sync,
            commands::get_active_academic_year,
            commands::get_active_session,
            commands::save_session,
            commands::logout,
            commands::get_today_cashier_stats,
            commands::get_today_payments,
            commands::is_savings_enabled,
            commands::get_school_settings,
            commands::get_payment_receipt,
            commands::test_sync_connection,
            commands::get_all_payment_history,
            commands::get_local_reports,
            commands::is_fresh_install,
            commands::download_school_assets,
            commands::get_pending_deep_link,
            commands::set_offline_pin,
            commands::verify_offline_pin,
            commands::has_offline_pin,
            commands::login_with_pairing_code,
            commands::get_pending_sync_count
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
