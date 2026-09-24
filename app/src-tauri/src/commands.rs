use library_domain::NewBook;
use tauri::State;

use crate::catalog::{self, BookSummary, CreatedBook, HistoryEntry};
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
pub fn book_history(
    database: State<'_, Database>,
    edition_id: String,
) -> Result<Vec<HistoryEntry>> {
    let conn = database.lock()?;
    catalog::book_history(&conn, &edition_id)
}
