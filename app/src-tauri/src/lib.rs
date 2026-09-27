mod backup;
mod catalog;
mod commands;
mod covers;
mod csv_io;
mod data_folder;
mod db;
mod error;
mod lan;
mod libraries;
mod marks;
mod metadata;
mod ops;
mod people;
mod permissions;
mod requests;
mod search;
mod settings;
mod sync;
mod wishes;

use tauri::Manager;

use crate::covers::Covers;
use crate::db::Database;
use crate::lan::identity::Identity;
use crate::lan::service::LanService;
use crate::metadata::Lookup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());
    builder
        .setup(|app| {
            let data_dir = data_dir(app.path().app_data_dir()?)?;
            std::fs::create_dir_all(&data_dir)?;
            let database = Database::open(data_dir.join(data_folder::DATABASE_FILE))?;
            let lan = LanService::new(database.clone(), Identity::load_or_create(&data_dir)?);
            app.manage(database);
            app.manage(Covers::new(data_dir.join("covers"))?);
            app.manage(Lookup::new()?);
            app.manage(lan.clone());
            // Devices paired earlier can sync as soon as the app is open.
            tauri::async_runtime::spawn(async move {
                let _ = lan.start_if_paired().await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_book,
            commands::list_books,
            commands::get_book,
            commands::update_edition,
            commands::delete_book,
            commands::add_copy,
            commands::update_copy,
            commands::delete_copy,
            commands::request_book,
            commands::request_delete_copy,
            commands::request_delete_book,
            commands::book_history,
            commands::lookup_isbn,
            commands::get_settings,
            commands::save_settings,
            commands::covers_dir,
            commands::catalog_options,
            commands::export_backup,
            commands::export_device_backup,
            commands::restore_backup,
            commands::export_csv,
            commands::import_csv,
            commands::lan_status,
            commands::set_device_name,
            commands::open_pairing,
            commands::close_pairing,
            commands::pair_with,
            commands::sync_with,
            commands::forget_device,
            commands::add_wish,
            commands::list_wishes,
            commands::delete_wish,
            commands::check_isbn,
            commands::is_mobile,
            commands::list_libraries,
            commands::create_library,
            commands::rename_library,
            commands::set_active_library,
            commands::delete_library,
            commands::leave_library,
            commands::move_book,
            commands::library_devices,
            commands::sharing_overview,
            commands::set_sharing,
            commands::list_requests,
            commands::accept_request,
            commands::reject_request,
            commands::withdraw_request,
            commands::requested_book,
            commands::accept_request_edited,
            commands::request_notices,
            commands::dismiss_request_notices,
            commands::list_people,
            commands::start_roles,
            commands::known_people,
            commands::pairing_owners,
            commands::give_access,
            commands::work_marks,
            commands::set_mark,
            commands::claim_person,
            commands::set_role,
            commands::assign_device,
            commands::remove_device,
            commands::duplicate_library
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// In debug builds `KOSMOTHEKA_DATA_DIR` points a second instance at its own data, so
/// two copies of the app on one computer can sync with each other. Otherwise the
/// app's own folder, carried over from the old identifier the first time (D70).
fn data_dir(default: std::path::PathBuf) -> error::Result<std::path::PathBuf> {
    #[cfg(debug_assertions)]
    if let Some(dir) = std::env::var_os("KOSMOTHEKA_DATA_DIR") {
        return Ok(dir.into());
    }
    if let Some(legacy) = data_folder::legacy_dir(&default) {
        data_folder::copy_legacy_data(&default, &legacy)?;
    }
    Ok(default)
}
