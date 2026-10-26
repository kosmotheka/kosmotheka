//! CSV export and import, one row per copy, for spreadsheets and other apps (D30): every kind
//! in one file, with a `kind` column. Several names (or genres, or tags) share one cell,
//! separated by semicolons.

use kosmotheka_domain::{
    ContributorInput, ContributorRole, CopyInput, EditionInput, ErrorCode, FieldError, Format,
    Kind, NewBook,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::catalog;
use crate::error::{Error, Result};

/// One copy per row; a column that does not apply to the row's kind stays empty. A file from
/// before the kinds (only the book's columns) still imports: every missing column is empty, and
/// an empty kind is a book.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Row {
    kind: String,
    title: String,
    subtitle: String,
    authors: String,
    writers: String,
    illustrators: String,
    designers: String,
    directors: String,
    artists: String,
    studios: String,
    publisher: String,
    published_year: String,
    language: String,
    isbn13: String,
    barcode: String,
    number: String,
    players_min: String,
    players_max: String,
    duration_minutes: String,
    min_age: String,
    seasons: String,
    genres: String,
    tags: String,
    format: String,
    owner: String,
    quantity: String,
    location: String,
    acquired_on: String,
    platform: String,
    copy_seasons: String,
}

/// The roles in their columns, `authors` first as it always was.
const ROLES: [(ContributorRole, &str); 7] = [
    (ContributorRole::Author, "author"),
    (ContributorRole::Writer, "writer"),
    (ContributorRole::Illustrator, "illustrator"),
    (ContributorRole::Designer, "designer"),
    (ContributorRole::Director, "director"),
    (ContributorRole::Artist, "artist"),
    (ContributorRole::Studio, "studio"),
];

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
pub fn export(conn: &Connection, library: &str, mut out: impl std::io::Write) -> Result<usize> {
    let people = |role: &str| {
        format!(
            "coalesce((SELECT group_concat(p.name, '; ' ORDER BY wc.position)
                         FROM work_contributors wc
                         JOIN people p ON p.library_id = wc.library_id AND p.id = wc.person_id
                        WHERE wc.library_id = e.library_id AND wc.work_id = e.work_id
                          AND wc.role = '{role}'), '')"
        )
    };
    let roles = ROLES.map(|(_, role)| people(role)).join(", ");
    let mut statement = conn.prepare(&format!(
        "SELECT w.kind, e.title, coalesce(e.subtitle, ''), {roles},
                coalesce(pu.name, ''), coalesce(e.published_year, ''), coalesce(e.language, ''),
                coalesce(bd.isbn13, ''), coalesce(e.barcode, ''), coalesce(cd.number, ''),
                coalesce(gd.players_min, ''), coalesce(gd.players_max, ''),
                coalesce(gd.duration_minutes, fd.duration_minutes, ''), coalesce(gd.min_age, ''),
                coalesce(td.seasons, ''),
                coalesce((SELECT group_concat(genre, '; ' ORDER BY genre) FROM work_genres
                           WHERE library_id = e.library_id AND work_id = e.work_id), ''),
                coalesce((SELECT group_concat(t.name, '; ' ORDER BY t.name COLLATE NOCASE)
                            FROM work_tags wt
                            JOIN tags t ON t.library_id = wt.library_id AND t.id = wt.tag_id
                           WHERE wt.library_id = e.library_id AND wt.work_id = e.work_id), ''),
                coalesce(c.format, ''), coalesce(m.name, ''), coalesce(c.quantity, ''),
                coalesce(c.location, ''), coalesce(c.acquired_on, ''), coalesce(c.platform, ''),
                coalesce(c.seasons, '')
           FROM editions e
           JOIN works w ON w.library_id = e.library_id AND w.id = e.work_id
           LEFT JOIN publishers pu ON pu.library_id = e.library_id AND pu.id = e.publisher_id
           LEFT JOIN book_details bd ON bd.library_id = e.library_id AND bd.edition_id = e.id
           LEFT JOIN comic_details cd ON cd.library_id = e.library_id AND cd.edition_id = e.id
           LEFT JOIN boardgame_details gd ON gd.library_id = e.library_id AND gd.edition_id = e.id
           LEFT JOIN film_details fd ON fd.library_id = e.library_id AND fd.edition_id = e.id
           LEFT JOIN tv_series_details td ON td.library_id = e.library_id AND td.edition_id = e.id
           LEFT JOIN copies c ON c.library_id = e.library_id AND c.edition_id = e.id
           LEFT JOIN members m ON m.library_id = c.library_id AND m.id = c.owner_id
          WHERE e.library_id = ?1
          ORDER BY e.title COLLATE NOCASE, e.id, c.created_at"
    ))?;
    let rows = statement.query_map([library], |row| {
        let mut column = 0;
        let mut next = || -> rusqlite::Result<String> {
            let value: rusqlite::types::Value = row.get(column)?;
            column += 1;
            Ok(match value {
                rusqlite::types::Value::Integer(n) => n.to_string(),
                rusqlite::types::Value::Text(s) => s,
                _ => String::new(),
            })
        };
        Ok(Row {
            kind: next()?,
            title: next()?,
            subtitle: next()?,
            authors: next()?,
            writers: next()?,
            illustrators: next()?,
            designers: next()?,
            directors: next()?,
            artists: next()?,
            studios: next()?,
            publisher: next()?,
            published_year: next()?,
            language: next()?,
            isbn13: next()?,
            barcode: next()?,
            number: next()?,
            players_min: next()?,
            players_max: next()?,
            duration_minutes: next()?,
            min_age: next()?,
            seasons: next()?,
            genres: next()?,
            tags: next()?,
            format: next()?,
            owner: next()?,
            quantity: next()?,
            location: next()?,
            acquired_on: next()?,
            platform: next()?,
            copy_seasons: next()?,
        })
    })?;

    // A byte order mark lets spreadsheet apps detect UTF-8 (accents, ñ).
    std::io::Write::write_all(&mut out, b"\xEF\xBB\xBF")?;
    let mut writer = csv::Writer::from_writer(out);
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
pub fn import(
    conn: &mut Connection,
    library: &str,
    mut input: impl std::io::Read,
) -> Result<CsvImportSummary> {
    let mut content = String::new();
    input.read_to_string(&mut content)?;
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
    let list = |value: &str| -> Vec<String> {
        value
            .split(';')
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect()
    };
    let mut number = |field: &str, value: &str| -> Option<i64> {
        match value {
            "" => None,
            text => text.parse().map(Some).unwrap_or_else(|_| {
                errors.push(FieldError::new(field, ErrorCode::OutOfRange));
                None
            }),
        }
    };

    let published_year =
        number("publishedYear", &row.published_year).and_then(|year| i32::try_from(year).ok());
    let players_min = number("playersMin", &row.players_min);
    let players_max = number("playersMax", &row.players_max);
    let duration_minutes = number("durationMinutes", &row.duration_minutes);
    let min_age = number("minAge", &row.min_age);
    let seasons = number("seasons", &row.seasons);
    let quantity = number("copy.quantity", &row.quantity)
        .map(|n| u32::try_from(n).unwrap_or(0))
        .unwrap_or(1);
    let kind = match row.kind.to_lowercase().as_str() {
        "" => Kind::Book,
        name => name.parse().unwrap_or_else(|_| {
            errors.push(FieldError::new("kind", ErrorCode::InvalidFormat));
            Kind::Book
        }),
    };
    // Copies need a format; a row without one is still a work worth keeping.
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

    let columns = [
        &row.authors,
        &row.writers,
        &row.illustrators,
        &row.designers,
        &row.directors,
        &row.artists,
        &row.studios,
    ];
    let contributors = ROLES
        .iter()
        .zip(columns)
        .skip(1)
        .flat_map(|((role, _), names)| {
            list(names)
                .into_iter()
                .map(|name| ContributorInput { name, role: *role })
        })
        .collect();

    Ok(NewBook {
        edition: EditionInput {
            kind,
            title: row.title,
            subtitle: optional(row.subtitle),
            authors: list(&row.authors),
            contributors,
            publisher: optional(row.publisher),
            published_year,
            language: optional(row.language),
            isbn: optional(row.isbn13),
            barcode: optional(row.barcode),
            number: optional(row.number),
            players_min,
            players_max,
            duration_minutes,
            min_age,
            seasons,
            genres: list(&row.genres),
            tags: list(&row.tags),
            ..EditionInput::default()
        },
        copy: CopyInput {
            owner: optional(row.owner),
            format,
            quantity,
            location: optional(row.location),
            acquired_on: optional(row.acquired_on),
            platform: optional(row.platform),
            seasons: optional(row.copy_seasons),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::BookFilter;
    use crate::db;
    use crate::ops;

    fn temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("library-csv-{name}-{}.csv", std::process::id()))
    }

    #[test]
    fn exported_csv_imports_back_into_the_same_catalogue() {
        let mut source = db::open_in_memory().unwrap();
        let from = ops::active_library(&source).unwrap();
        let path = temp_file("roundtrip");
        std::fs::write(
            &path,
            "title,authors,publisher,published_year,isbn13,format,owner,quantity,location\n\
             El señor de los anillos,J. R. R. Tolkien,Minotauro,1954,9780306406157,hardcover,Ana,1,\"Salón, balda 2\"\n\
             El señor de los anillos,J. R. R. Tolkien,Minotauro,1954,9780306406157,ebook,Bea,,\n\
             Good Omens,Terry Pratchett; Neil Gaiman,,,,,,,\n",
        )
        .unwrap();
        let imported = import(&mut source, &from, std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(
            imported,
            CsvImportSummary {
                books_created: 2,
                copies_added: 1,
                rejected: vec![]
            }
        );

        export(&source, &from, std::fs::File::create(&path).unwrap()).unwrap();
        let exported = std::fs::read_to_string(&path).unwrap();
        assert!(exported.starts_with('\u{feff}'));
        assert!(exported.contains("\"Salón, balda 2\""));
        assert!(exported.contains("Terry Pratchett; Neil Gaiman"));

        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        let reimported = import(&mut target, &to, std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(reimported, imported);
        let books = catalog::list_books(&target, &to, &BookFilter::default()).unwrap();
        assert_eq!(books[1].authors, vec!["Terry Pratchett", "Neil Gaiman"]);
        assert_eq!(books[0].copy_count, 2);
        assert_eq!(books[0].formats, vec![Format::Hardcover, Format::Ebook]);
    }

    #[test]
    fn every_kind_goes_out_and_comes_back_with_its_own_columns() {
        let mut source = db::open_in_memory().unwrap();
        let from = ops::active_library(&source).unwrap();
        let path = temp_file("kinds");
        std::fs::write(
            &path,
            "kind,title,designers,publisher,barcode,players_min,players_max,duration_minutes,min_age,genres,tags,format,platform,copy_seasons,seasons,directors\n\
             boardgame,Catan,Klaus Teuber,Devir,4006381333931,3,4,75,10,strategy; family,para jugar en casa,box,,,,\n\
             videogame,Zelda,,Nintendo,,,,,,,,physical,Switch,,,\n\
             tv_series,Twin Peaks,,,,,,,,,,dvd,,1-2,3,David Lynch\n",
        )
        .unwrap();
        let imported = import(&mut source, &from, std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(imported.rejected, vec![]);
        assert_eq!(imported.books_created, 3);

        export(&source, &from, std::fs::File::create(&path).unwrap()).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        let reimported = import(&mut target, &to, std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(reimported, imported);

        let books = catalog::list_books(&target, &to, &BookFilter::default()).unwrap();
        let detail = |title: &str| {
            let summary = books.iter().find(|b| b.title == title).unwrap();
            catalog::get_book(&target, &to, &summary.edition_id).unwrap()
        };
        let catan = detail("Catan");
        assert_eq!(catan.kind, Kind::Boardgame);
        assert_eq!(catan.barcode.as_deref(), Some("4006381333931"));
        assert_eq!((catan.players_min, catan.players_max), (Some(3), Some(4)));
        assert_eq!(
            (catan.duration_minutes, catan.min_age),
            (Some(75), Some(10))
        );
        assert_eq!(catan.genres, ["family", "strategy"]);
        assert_eq!(catan.contributors[0].name, "Klaus Teuber");
        assert_eq!(catan.copies[0].format, Format::Box);
        assert_eq!(
            detail("Zelda").copies[0].platform.as_deref(),
            Some("Switch")
        );
        let peaks = detail("Twin Peaks");
        assert_eq!(peaks.seasons, Some(3));
        assert_eq!(peaks.copies[0].seasons.as_deref(), Some("1-2"));
        assert_eq!(peaks.contributors[0].name, "David Lynch");
    }

    #[test]
    fn invalid_rows_are_reported_and_the_rest_imported() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
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

        let summary = import(&mut conn, &lib, std::fs::File::open(&path).unwrap()).unwrap();

        assert_eq!(summary.books_created, 1);
        let lines: Vec<usize> = summary.rejected.iter().map(|r| r.line).collect();
        assert_eq!(lines, vec![3, 4, 5]);
        assert_eq!(summary.rejected[1].errors[0].code, ErrorCode::IsbnChecksum);
        assert_eq!(summary.rejected[2].errors.len(), 2);
    }
}
