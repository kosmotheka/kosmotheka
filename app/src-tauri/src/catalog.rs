use std::collections::HashMap;

use library_domain::{Change, Entity, Format, Hlc, ValidBook};
use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::Serialize;
use serde_json::json;

use crate::error::Result;
use crate::ops::{self, new_id};

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub operation_id: String,
    pub intent: String,
    /// Milliseconds since the Unix epoch, from the operation's HLC.
    pub timestamp: u64,
}

pub fn create_book(conn: &mut Connection, book: &ValidBook) -> Result<CreatedBook> {
    let tx = conn.transaction()?;
    let mut plan = Plan::new(&tx);

    let existing_edition_id: Option<String> = match &book.isbn {
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
        None => plan.add_edition(book)?,
    };

    let owner_id = match &book.copy.owner {
        Some(name) => Some(plan.find_or_create(Entity::Member, name)?),
        None => None,
    };
    plan.changes.push(Change::create(
        Entity::Copy,
        new_id(),
        json!({
            "edition_id": edition_id,
            "owner_id": owner_id,
            "format": book.copy.format.as_str(),
            "quantity": book.copy.quantity,
            "location": book.copy.location,
            "acquired_on": book.copy.acquired_on.map(|date| date.to_string()),
        }),
    ));

    let intent = if existing_edition {
        "copy.add"
    } else {
        "book.add"
    };
    let changes = plan.changes;
    ops::record(&tx, intent, changes)?;
    tx.commit()?;
    Ok(CreatedBook {
        edition_id,
        existing_edition,
    })
}

/// Collects the changes of one operation, reusing rows that already exist.
struct Plan<'a> {
    tx: &'a Transaction<'a>,
    changes: Vec<Change>,
    /// Rows created earlier in this same plan, so a repeated name is created once.
    planned: HashMap<(Entity, String), String>,
}

impl<'a> Plan<'a> {
    fn new(tx: &'a Transaction<'a>) -> Self {
        Self {
            tx,
            changes: Vec::new(),
            planned: HashMap::new(),
        }
    }

    fn add_edition(&mut self, book: &ValidBook) -> Result<String> {
        let work_id = new_id();
        self.changes.push(Change::create(
            Entity::Work,
            &work_id,
            json!({ "kind": "book", "title": book.title }),
        ));

        let mut author_ids: Vec<String> = Vec::new();
        for author in &book.authors {
            let person_id = self.find_or_create(Entity::Person, author)?;
            if !author_ids.contains(&person_id) {
                author_ids.push(person_id);
            }
        }
        for (position, person_id) in author_ids.iter().enumerate() {
            self.changes.push(Change::create(
                Entity::WorkContributor,
                new_id(),
                json!({ "work_id": work_id, "person_id": person_id, "role": "author", "position": position }),
            ));
        }

        let edition_id = new_id();
        self.changes.push(Change::create(
            Entity::Edition,
            &edition_id,
            json!({
                "work_id": work_id,
                "title": book.title,
                "subtitle": book.subtitle,
                "publisher": book.publisher,
                "published_year": book.published_year,
                "language": book.language,
            }),
        ));
        self.changes.push(Change::create(
            Entity::BookDetails,
            &edition_id,
            json!({ "isbn13": book.isbn.as_ref().map(|isbn| isbn.as_str()) }),
        ));
        Ok(edition_id)
    }

    /// For entities identified by a `name` (people, members), matched case-insensitively.
    fn find_or_create(&mut self, entity: Entity, name: &str) -> Result<String> {
        let key = (entity, name.to_lowercase());
        if let Some(id) = self.planned.get(&key) {
            return Ok(id.clone());
        }
        let table = entity.table();
        let existing: Option<String> = self
            .tx
            .query_row(
                &format!("SELECT id FROM {table} WHERE name = ?1 COLLATE NOCASE"),
                [name],
                |row| row.get(0),
            )
            .optional()?;
        let id = match existing {
            Some(id) => id,
            None => {
                let id = new_id();
                self.changes
                    .push(Change::create(entity, &id, json!({ "name": name })));
                id
            }
        };
        self.planned.insert(key, id.clone());
        Ok(id)
    }
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

/// Operations that touched the edition, its work or any of its copies, oldest first.
pub fn book_history(conn: &Connection, edition_id: &str) -> Result<Vec<HistoryEntry>> {
    let mut statement = conn.prepare(
        "WITH related (id) AS (
             SELECT ?1
             UNION SELECT work_id FROM editions WHERE id = ?1
             UNION SELECT id FROM copies WHERE edition_id = ?1
         )
         SELECT DISTINCT o.id, o.intent, o.hlc
           FROM operations o, json_each(o.changes) AS change
          WHERE json_extract(change.value, '$.id') IN (SELECT id FROM related)
          ORDER BY o.hlc",
    )?;
    let rows = statement.query_map([edition_id], |row| {
        let hlc: String = row.get(2)?;
        Ok(HistoryEntry {
            operation_id: row.get(0)?,
            intent: row.get(1)?,
            timestamp: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn split_list(joined: Option<String>) -> Vec<String> {
    joined
        .map(|s| s.split(LIST_SEPARATOR).map(str::to_owned).collect())
        .unwrap_or_default()
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

    #[test]
    fn repeated_author_in_one_book_is_created_once() {
        let mut conn = db::open_in_memory().unwrap();
        let mut book = new_book("C", None, "Tersoal", Format::Paperback);
        book.authors = vec!["Ursula K. Le Guin".into(), "ursula k. le guin".into()];

        create_book(&mut conn, &book).unwrap();

        assert_eq!(
            list_books(&conn).unwrap()[0].authors,
            vec!["Ursula K. Le Guin"]
        );
    }

    #[test]
    fn every_write_is_logged_and_history_follows_the_edition() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
        )
        .unwrap();
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Ebook),
        )
        .unwrap();
        create_book(
            &mut conn,
            &new_book("Other", None, "Tersoal", Format::Ebook),
        )
        .unwrap();

        let history = book_history(&conn, &created.edition_id).unwrap();

        let intents: Vec<&str> = history.iter().map(|entry| entry.intent.as_str()).collect();
        assert_eq!(intents, vec!["book.add", "copy.add"]);
        assert!(history[0].timestamp <= history[1].timestamp);
    }
}
