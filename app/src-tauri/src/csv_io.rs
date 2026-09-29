//! CSV export and import, one row per copy, for spreadsheets and other apps (D30).
//! Several authors share one cell, separated by semicolons.

use std::path::Path;

use kosmotheka_domain::{CopyInput, EditionInput, ErrorCode, FieldError, Format, NewBook};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::catalog;
use crate::error::{Error, Result};

#[derive(Debug, Serialize, Deserialize)]
struct Row {
    title: String,
    #[serde(default)]
    subtitle: String,
    #[serde(default)]
    authors: String,
    #[serde(default)]
    publisher: String,
    #[serde(default)]
    published_year: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    isbn13: String,
    #[serde(default)]
    format: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    quantity: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    acquired_on: String,
}

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportSummary {
    pub books_created: usize,
    pub copies_added: usize,
    pub rejected: Vec<RejectedRow>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedRow {
    /// Line in the file, counting the header as line 1.
    pub line: usize,
    pub errors: Vec<FieldError>,
}

/// Returns the number of data rows written. Editions without copies get one row
/// with the copy columns empty.
pub fn export(conn: &Connection, library: &str, path: &Path) -> Result<usize> {
    let mut statement = conn.prepare(
        "SELECT e.title, coalesce(e.subtitle, ''),
                coalesce((SELECT group_concat(p.name, '; ' ORDER BY wc.position)
                            FROM work_contributors wc
                            JOIN people p ON p.library_id = wc.library_id AND p.id = wc.person_id
                           WHERE wc.library_id = e.library_id AND wc.work_id = e.work_id
                             AND wc.role = 'author'), ''),
                coalesce(pu.name, ''), coalesce(e.published_year, ''), coalesce(e.language, ''),
                coalesce(bd.isbn13, ''), coalesce(c.format, ''), coalesce(m.name, ''),
                coalesce(c.quantity, ''), coalesce(c.location, ''), coalesce(c.acquired_on, '')
           FROM editions e
           LEFT JOIN publishers pu ON pu.library_id = e.library_id AND pu.id = e.publisher_id
           LEFT JOIN book_details bd ON bd.library_id = e.library_id AND bd.edition_id = e.id
           LEFT JOIN copies c ON c.library_id = e.library_id AND c.edition_id = e.id
           LEFT JOIN members m ON m.library_id = c.library_id AND m.id = c.owner_id
          WHERE e.library_id = ?1
          ORDER BY e.title COLLATE NOCASE, e.id, c.created_at",
    )?;
    let rows = statement.query_map([library], |row| {
        let text = |i: usize| -> rusqlite::Result<String> {
            let value: rusqlite::types::Value = row.get(i)?;
            Ok(match value {
                rusqlite::types::Value::Integer(n) => n.to_string(),
                rusqlite::types::Value::Text(s) => s,
                _ => String::new(),
            })
        };
        Ok(Row {
            title: text(0)?,
            subtitle: text(1)?,
            authors: text(2)?,
            publisher: text(3)?,
            published_year: text(4)?,
            language: text(5)?,
            isbn13: text(6)?,
            format: text(7)?,
            owner: text(8)?,
            quantity: text(9)?,
            location: text(10)?,
            acquired_on: text(11)?,
        })
    })?;

    // A byte order mark lets spreadsheet apps detect UTF-8 (accents, ñ).
    let mut file = std::fs::File::create(path)?;
    std::io::Write::write_all(&mut file, b"\xEF\xBB\xBF")?;
    let mut writer = csv::Writer::from_writer(file);
    let mut written = 0;
    for row in rows {
        writer.serialize(row?)?;
        written += 1;
    }
    writer.flush()?;
    Ok(written)
}

/// Adds every valid row as a book (or as a copy, if its ISBN is already catalogued).
/// Invalid rows are reported and skipped; the rest are still imported.
pub fn import(conn: &mut Connection, library: &str, path: &Path) -> Result<CsvImportSummary> {
    let content = std::fs::read_to_string(path)?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());

    let mut summary = CsvImportSummary::default();
    for (index, record) in reader.deserialize::<Row>().enumerate() {
        let line = index + 2;
        let row = match record {
            Ok(row) => row,
            Err(_) => {
                summary.rejected.push(RejectedRow {
                    line,
                    errors: vec![FieldError::new("row", ErrorCode::Required)],
                });
                continue;
            }
        };
        let book = match to_book(row).and_then(|book| book.validate()) {
            Ok(book) => book,
            Err(errors) => {
                summary.rejected.push(RejectedRow { line, errors });
                continue;
            }
        };
        match catalog::create_book(conn, library, &book, None) {
            Ok(created) if created.existing_edition => summary.copies_added += 1,
            Ok(_) => summary.books_created += 1,
            Err(Error::Validation(errors)) => summary.rejected.push(RejectedRow { line, errors }),
            Err(other) => return Err(other),
        }
    }
    Ok(summary)
}

fn to_book(row: Row) -> std::result::Result<NewBook, Vec<FieldError>> {
    let mut errors = Vec::new();
    let optional = |value: String| Some(value).filter(|v| !v.is_empty());

    let published_year = match row.published_year.as_str() {
        "" => None,
        year => year.parse().map(Some).unwrap_or_else(|_| {
            errors.push(FieldError::new("publishedYear", ErrorCode::OutOfRange));
            None
        }),
    };
    let quantity = match row.quantity.as_str() {
        "" => 1,
        quantity => quantity.parse().unwrap_or_else(|_| {
            errors.push(FieldError::new("copy.quantity", ErrorCode::OutOfRange));
            1
        }),
    };
    // Copies need a format; a row without one is still a book worth keeping.
    let format = match row.format.to_lowercase().as_str() {
        "" => Format::Other,
        name => name.parse().unwrap_or_else(|_| {
            errors.push(FieldError::new("copy.format", ErrorCode::InvalidFormat));
            Format::Other
        }),
    };
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(NewBook {
        edition: EditionInput {
            title: row.title,
            subtitle: optional(row.subtitle),
            authors: row
                .authors
                .split(';')
                .map(|a| a.trim().to_owned())
                .collect(),
            publisher: optional(row.publisher),
            published_year,
            language: optional(row.language),
            isbn: optional(row.isbn13),
            ..EditionInput::default()
        },
        copy: CopyInput {
            owner: optional(row.owner),
            format,
            quantity,
            location: optional(row.location),
            acquired_on: optional(row.acquired_on),
            platform: None,
            seasons: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::BookFilter;
    use crate::db;
    use crate::ops::DEFAULT_LIBRARY_ID as LIB;

    fn temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("library-csv-{name}-{}.csv", std::process::id()))
    }

    #[test]
    fn exported_csv_imports_back_into_the_same_catalogue() {
        let mut source = db::open_in_memory().unwrap();
        let path = temp_file("roundtrip");
        std::fs::write(
            &path,
            "title,authors,publisher,published_year,isbn13,format,owner,quantity,location\n\
             El señor de los anillos,J. R. R. Tolkien,Minotauro,1954,9780306406157,hardcover,Ana,1,\"Salón, balda 2\"\n\
             El señor de los anillos,J. R. R. Tolkien,Minotauro,1954,9780306406157,ebook,Bea,,\n\
             Good Omens,Terry Pratchett; Neil Gaiman,,,,,,,\n",
        )
        .unwrap();
        let imported = import(&mut source, LIB, &path).unwrap();
        assert_eq!(
            imported,
            CsvImportSummary {
                books_created: 2,
                copies_added: 1,
                rejected: vec![]
            }
        );

        export(&source, LIB, &path).unwrap();
        let exported = std::fs::read_to_string(&path).unwrap();
        assert!(exported.starts_with('\u{feff}'));
        assert!(exported.contains("\"Salón, balda 2\""));
        assert!(exported.contains("Terry Pratchett; Neil Gaiman"));

        let mut target = db::open_in_memory().unwrap();
        let reimported = import(&mut target, LIB, &path).unwrap();
        assert_eq!(reimported, imported);
        let books = catalog::list_books(&target, LIB, &BookFilter::default()).unwrap();
        assert_eq!(books[1].authors, vec!["Terry Pratchett", "Neil Gaiman"]);
        assert_eq!(books[0].copy_count, 2);
        assert_eq!(books[0].formats, vec![Format::Hardcover, Format::Ebook]);
    }

    #[test]
    fn invalid_rows_are_reported_and_the_rest_imported() {
        let mut conn = db::open_in_memory().unwrap();
        let path = temp_file("invalid");
        std::fs::write(
            &path,
            "title,isbn13,format,published_year\n\
             Good,,paperback,\n\
             ,,,\n\
             Bad ISBN,9780306406158,,\n\
             Bad format,,scroll,abc\n",
        )
        .unwrap();

        let summary = import(&mut conn, LIB, &path).unwrap();

        assert_eq!(summary.books_created, 1);
        let lines: Vec<usize> = summary.rejected.iter().map(|r| r.line).collect();
        assert_eq!(lines, vec![3, 4, 5]);
        assert_eq!(summary.rejected[1].errors[0].code, ErrorCode::IsbnChecksum);
        assert_eq!(summary.rejected[2].errors.len(), 2);
    }
}
