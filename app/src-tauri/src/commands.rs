use library_domain::{CopyInput, EditionInput, NewBook};
use tauri::State;

use crate::catalog::{self, BookDetail, BookSummary, CreatedBook, HistoryEntry};
use crate::db::Database;
use crate::error::{Error, Result};

#[tauri::command]
pub fn create_book(database: State<'_, Database>, book: NewBook) -> Result<CreatedBook> {
    let book = book.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    catalog::create_book(&mut conn, &book)
}

#[tauri::command]
pub fn list_books(database: State<'_, Database>) -> Result<Vec<BookSummary>> {
    let conn = database.lock()?;
    catalog::list_books(&conn)
}

#[tauri::command]
pub fn get_book(database: State<'_, Database>, edition_id: String) -> Result<BookDetail> {
    let conn = database.lock()?;
    catalog::get_book(&conn, &edition_id)
}

#[tauri::command]
pub fn update_edition(
    database: State<'_, Database>,
    edition_id: String,
    edition: EditionInput,
) -> Result<()> {
    let edition = edition.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    catalog::update_edition(&mut conn, &edition_id, &edition)
}

#[tauri::command]
pub fn delete_book(database: State<'_, Database>, edition_id: String) -> Result<()> {
    let mut conn = database.lock()?;
    catalog::delete_book(&mut conn, &edition_id)
}

#[tauri::command]
pub fn add_copy(
    database: State<'_, Database>,
    edition_id: String,
    copy: CopyInput,
) -> Result<String> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    catalog::add_copy(&mut conn, &edition_id, &copy)
}

#[tauri::command]
pub fn update_copy(database: State<'_, Database>, copy_id: String, copy: CopyInput) -> Result<()> {
    let copy = copy.validate().map_err(Error::Validation)?;
    let mut conn = database.lock()?;
    catalog::update_copy(&mut conn, &copy_id, &copy)
}

#[tauri::command]
pub fn delete_copy(database: State<'_, Database>, copy_id: String) -> Result<()> {
    let mut conn = database.lock()?;
    catalog::delete_copy(&mut conn, &copy_id)
}

#[tauri::command]
pub fn book_history(
    database: State<'_, Database>,
    edition_id: String,
) -> Result<Vec<HistoryEntry>> {
    let conn = database.lock()?;
    catalog::book_history(&conn, &edition_id)
}
