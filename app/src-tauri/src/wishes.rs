//! The wishlist (D36) and the offline "do I already have it?" check.

use library_domain::{Change, Entity, FieldError, Isbn, ValidWish};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::json;

use crate::catalog::{self, BookFilter, BookSummary};
use crate::error::{Error, Result};
use crate::ops::{self, new_id};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wish {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub isbn13: Option<String>,
    pub note: Option<String>,
    pub cover_url: Option<String>,
    pub created_at: String,
}

/// What the catalog knows about an ISBN, from the local database only, so it works
/// in a shop without coverage.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsbnCheck {
    pub isbn13: String,
    pub books: Vec<BookSummary>,
    pub wishes: Vec<Wish>,
}

pub fn add_wish(conn: &mut Connection, library: &str, wish: &ValidWish) -> Result<String> {
    let tx = conn.transaction()?;
    let id = new_id();
    ops::record(
        &tx,
        library,
        "wish.add",
        vec![Change::create(
            Entity::Wish,
            &id,
            json!({
                "title": wish.title,
                "author": wish.author,
                "isbn13": wish.isbn.as_ref().map(|isbn| isbn.as_str()),
                "note": wish.note,
                "cover_url": wish.cover_url.as_ref().map(|url| url.as_str()),
            }),
        )],
    )?;
    tx.commit()?;
    Ok(id)
}

pub fn delete_wish(conn: &mut Connection, library: &str, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        library,
        "wish.remove",
        vec![Change::delete(Entity::Wish, id)],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list_wishes(conn: &Connection, library: &str) -> Result<Vec<Wish>> {
    query_wishes(
        conn,
        "SELECT id, title, author, isbn13, note, cover_url, created_at FROM wishes
          WHERE library_id = ?1 ORDER BY created_at DESC, id",
        [library],
    )
}

pub fn check_isbn(conn: &Connection, library: &str, raw: &str) -> Result<IsbnCheck> {
    let isbn = Isbn::parse(raw)
        .map_err(|error| Error::Validation(vec![FieldError::new("isbn", error.into())]))?;
    let filter = BookFilter {
        query: Some(isbn.as_str().to_owned()),
        ..BookFilter::default()
    };
    Ok(IsbnCheck {
        isbn13: isbn.as_str().to_owned(),
        books: catalog::list_books(conn, library, &filter)?,
        wishes: query_wishes(
            conn,
            "SELECT id, title, author, isbn13, note, cover_url, created_at FROM wishes
              WHERE library_id = ?1 AND isbn13 = ?2 ORDER BY created_at",
            [library, isbn.as_str()],
        )?,
    })
}

fn query_wishes<P: rusqlite::Params>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Wish>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params, |row| {
        Ok(Wish {
            id: row.get(0)?,
            title: row.get(1)?,
            author: row.get(2)?,
            isbn13: row.get(3)?,
            note: row.get(4)?,
            cover_url: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use library_domain::{CopyInput, EditionInput, Format, ValidBook, WishInput};

    use super::*;
    use crate::db;
    use crate::ops::DEFAULT_LIBRARY_ID as LIB;

    fn wish(title: &str, isbn: Option<&str>) -> ValidWish {
        WishInput {
            title: title.into(),
            isbn: isbn.map(str::to_owned),
            ..WishInput::default()
        }
        .validate()
        .unwrap()
    }

    #[test]
    fn wishes_are_added_listed_and_removed() {
        let mut conn = db::open_in_memory().unwrap();
        let id = add_wish(&mut conn, LIB, &wish("Tehanu", None)).unwrap();
        add_wish(&mut conn, LIB, &wish("The Other Wind", None)).unwrap();

        assert_eq!(list_wishes(&conn, LIB).unwrap().len(), 2);
        delete_wish(&mut conn, LIB, &id).unwrap();
        let titles: Vec<String> = list_wishes(&conn, LIB)
            .unwrap()
            .into_iter()
            .map(|w| w.title)
            .collect();
        assert_eq!(titles, vec!["The Other Wind"]);
    }

    #[test]
    fn check_finds_books_and_wishes_by_isbn_in_any_format() {
        let mut conn = db::open_in_memory().unwrap();
        let book = ValidBook {
            edition: EditionInput {
                title: "Owned".into(),
                isbn: Some("9780306406157".into()),
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: Some("Ana".into()),
                format: Format::Ebook,
                quantity: 1,
                location: None,
                acquired_on: None,
            }
            .validate()
            .unwrap(),
        };
        catalog::create_book(&mut conn, LIB, &book, None).unwrap();
        add_wish(&mut conn, LIB, &wish("Wanted", Some("9780140328721"))).unwrap();

        let owned = check_isbn(&conn, LIB, "0-306-40615-2").unwrap();
        assert_eq!(owned.books.len(), 1);
        assert_eq!(owned.books[0].owners, vec!["Ana"]);

        let wanted = check_isbn(&conn, LIB, "9780140328721").unwrap();
        assert!(wanted.books.is_empty());
        assert_eq!(wanted.wishes.len(), 1);

        assert!(matches!(
            check_isbn(&conn, LIB, "12"),
            Err(Error::Validation(_))
        ));
    }
}
