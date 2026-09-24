use tauri::State;

use crate::db::{self, AppInfo, Database};
use crate::error::Result;

#[tauri::command]
pub fn get_app_info(database: State<'_, Database>) -> Result<AppInfo> {
    let conn = database.lock()?;
    db::app_info(&conn, database.path())
}
