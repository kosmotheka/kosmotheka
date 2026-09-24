mod backup;
mod catalog;
mod commands;
mod covers;
mod csv_io;
mod db;
mod error;
mod metadata;
mod ops;
mod search;
mod settings;

use tauri::Manager;

use crate::covers::Covers;
use crate::db::Database;
use crate::metadata::Lookup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(Database::open(data_dir.join("library.sqlite3"))?);
            app.manage(Covers::new(data_dir.join("covers"))?);
            app.manage(Lookup::new()?);
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
            commands::book_history,
            commands::lookup_isbn,
            commands::get_settings,
            commands::save_settings,
            commands::covers_dir,
            commands::catalog_options,
            commands::export_backup,
            commands::restore_backup,
            commands::export_csv,
            commands::import_csv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
