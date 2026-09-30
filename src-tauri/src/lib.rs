mod db;
mod migrations;
mod sync;
mod commands;

use std::sync::Mutex;
use tauri::{Manager, Emitter};
use tauri_plugin_deep_link::DeepLinkExt;

pub struct PendingDeepLink(pub Mutex<Option<String>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
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
            
            Ok(())
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
