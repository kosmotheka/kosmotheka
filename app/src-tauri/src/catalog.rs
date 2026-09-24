use library_domain::{Format, ValidBook};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use uuid::Uuid;

use crate::error::Result;

// group_concat separator that cannot appear in names typed by users.
const LIST_SEPARATOR: char = '\u{1f}';

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedBook {
    pub edition_id: String,
    /// True when the ISBN was already catalogued and only a new copy was added.
    pub existing_edition: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSummary {
    pub edition_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub isbn13: Option<String>,
    pub copy_count: i64,
    pub formats: Vec<Format>,
    pub owners: Vec<String>,
}

pub fn create_book(conn: &mut Connection, book: &ValidBook) -> Result<CreatedBook> {
    let tx = conn.transaction()?;

    let existing_edition_id = match &book.isbn {
        Some(isbn) => tx
            .query_row(
                "SELECT edition_id FROM book_details WHERE isbn13 = ?1",
                [isbn.as_str()],
                |row| row.get(0),
            )
            .optional()?,
        None => None,
    };
    let existing_edition = existing_edition_id.is_some();
    let edition_id = match existing_edition_id {
        Some(id) => id,
        None => insert_edition(&tx, book)?,
    };

    let owner_id = match &book.copy.owner {
        Some(name) => Some(find_or_insert_by_name(&tx, "members", name)?),
        None => None,
    };
    tx.execute(
        "INSERT INTO copies (id, edition_id, owner_id, format, quantity, location, acquired_on)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            new_id(),
            edition_id,
            owner_id,
            book.copy.format.as_str(),
            book.copy.quantity,
            book.copy.location,
            book.copy.acquired_on.map(|date| date.to_string()),
        ],
    )?;

    tx.commit()?;
    Ok(CreatedBook {
        edition_id,
        existing_edition,
    })
}

fn insert_edition(tx: &Transaction<'_>, book: &ValidBook) -> Result<String> {
    let work_id = new_id();
    tx.execute(
        "INSERT INTO works (id, kind, title) VALUES (?1, 'book', ?2)",
        params![work_id, book.title],
    )?;
    for (position, author) in book.authors.iter().enumerate() {
        let person_id = find_or_insert_by_name(tx, "people", author)?;
        tx.execute(
            "INSERT OR IGNORE INTO work_contributors (work_id, person_id, role, position)
             VALUES (?1, ?2, 'author', ?3)",
            params![work_id, person_id, position as i64],
        )?;
    }

    let edition_id = new_id();
    tx.execute(
        "INSERT INTO editions (id, work_id, title, subtitle, publisher, published_year, language)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            edition_id,
            work_id,
            book.title,
            book.subtitle,
            book.publisher,
            book.published_year,
            book.language,
        ],
    )?;
    tx.execute(
        "INSERT INTO book_details (edition_id, isbn13) VALUES (?1, ?2)",
        params![edition_id, book.isbn.as_ref().map(|isbn| isbn.as_str())],
    )?;
    Ok(edition_id)
}

/// `table` is always a literal from this module, never user input.
fn find_or_insert_by_name(tx: &Transaction<'_>, table: &str, name: &str) -> Result<String> {
    let existing: Option<String> = tx
        .query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1 COLLATE NOCASE"),
            [name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = new_id();
    tx.execute(
        &format!("INSERT INTO {table} (id, name) VALUES (?1, ?2)"),
        params![id, name],
    )?;
    Ok(id)
}

pub fn list_books(conn: &Connection) -> Result<Vec<BookSummary>> {
    let mut statement = conn.prepare(
        "SELECT e.id, e.title, e.subtitle, e.publisher, e.published_year, bd.isbn13,
            (SELECT group_concat(p.name, char(31) ORDER BY wc.position)
               FROM work_contributors wc JOIN people p ON p.id = wc.person_id
              WHERE wc.work_id = e.work_id AND wc.role = 'author'),
            (SELECT coalesce(sum(c.quantity), 0) FROM copies c WHERE c.edition_id = e.id),
            (SELECT group_concat(format, char(31))
               FROM (SELECT DISTINCT c.format FROM copies c WHERE c.edition_id = e.id)),
            (SELECT group_concat(name, char(31) ORDER BY name COLLATE NOCASE)
               FROM (SELECT DISTINCT m.name FROM copies c JOIN members m ON m.id = c.owner_id
                      WHERE c.edition_id = e.id))
         FROM editions e
         LEFT JOIN book_details bd ON bd.edition_id = e.id
         ORDER BY e.title COLLATE NOCASE, e.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(BookSummary {
            edition_id: row.get(0)?,
            title: row.get(1)?,
            subtitle: row.get(2)?,
            publisher: row.get(3)?,
            published_year: row.get(4)?,
            isbn13: row.get(5)?,
            authors: split_list(row.get(6)?),
            copy_count: row.get(7)?,
            formats: split_list(row.get(8)?)
                .iter()
                .filter_map(|format| format.parse().ok())
                .collect(),
            owners: split_list(row.get(9)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn split_list(joined: Option<String>) -> Vec<String> {
    joined
        .map(|s| s.split(LIST_SEPARATOR).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use library_domain::{NewBook, NewCopy};

    use super::*;
    use crate::db;

    fn new_book(title: &str, isbn: Option<&str>, owner: &str, format: Format) -> ValidBook {
        NewBook {
            title: title.to_owned(),
            subtitle: None,
            authors: vec!["Patrick Rothfuss".to_owned()],
            publisher: None,
            published_year: Some(2007),
            language: Some("es".to_owned()),
            isbn: isbn.map(str::to_owned),
            copy: NewCopy {
                owner: Some(owner.to_owned()),
                format,
                quantity: 1,
                location: None,
                acquired_on: None,
            },
        }
        .validate()
        .unwrap()
    }

    #[test]
    fn creates_work_edition_and_copy() {
        let mut conn = db::open_in_memory().unwrap();
        let book = new_book(
            "El nombre del viento",
            Some("0306406152"),
            "Tersoal",
            Format::Hardcover,
        );

        let created = create_book(&mut conn, &book).unwrap();
        let books = list_books(&conn).unwrap();

        assert!(!created.existing_edition);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "El nombre del viento");
        assert_eq!(books[0].authors, vec!["Patrick Rothfuss"]);
        assert_eq!(books[0].isbn13.as_deref(), Some("9780306406157"));
        assert_eq!(books[0].formats, vec![Format::Hardcover]);
        assert_eq!(books[0].owners, vec!["Tersoal"]);
    }

    #[test]
    fn same_isbn_adds_a_copy_to_the_existing_edition() {
        let mut conn = db::open_in_memory().unwrap();
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
        )
        .unwrap();

        let second = create_book(
            &mut conn,
            &new_book("A", Some("0-306-40615-2"), "tersoal", Format::Ebook),
        )
        .unwrap();
        let books = list_books(&conn).unwrap();

        assert!(second.existing_edition);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].copy_count, 2);
        assert_eq!(
            books[0].owners,
            vec!["Tersoal"],
            "owners match case-insensitively"
        );
    }

    #[test]
    fn books_without_isbn_are_never_merged() {
        let mut conn = db::open_in_memory().unwrap();
        create_book(
            &mut conn,
            &new_book("B", None, "Tersoal", Format::Paperback),
        )
        .unwrap();
        create_book(
            &mut conn,
            &new_book("B", None, "Tersoal", Format::Paperback),
        )
        .unwrap();

        assert_eq!(list_books(&conn).unwrap().len(), 2);
    }
}
