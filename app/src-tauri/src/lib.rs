mod catalog;
mod commands;
mod db;
mod error;

use tauri::Manager;

use crate::db::Database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(Database::open(data_dir.join("library.sqlite3"))?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_book,
            commands::list_books
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
