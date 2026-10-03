use std::collections::HashMap;

use kosmotheka_domain::{
    Change, ContributorInput, ContributorRole, CopyInput, DETAILS, EditionInput, Entity, ErrorCode,
    FieldError, Format, Hlc, Isbn, Kind, MAX_TAG_LENGTH, ValidBook, ValidCopy, ValidEdition,
};
use rusqlite::params_from_iter;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::ops::{self, new_id};
use crate::search;

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
    /// The library the book is in: searches across libraries mix them.
    pub library_id: String,
    pub kind: Kind,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub isbn13: Option<String>,
    /// File name inside the covers directory.
    pub cover: Option<String>,
    pub copy_count: i64,
    pub formats: Vec<Format>,
    pub owners: Vec<String>,
    /// This device's person's status for the work (D68), if any.
    pub my_status: Option<String>,
    pub my_favorite: bool,
    pub my_wished: bool,
    /// Names of the members who wished the work (D61): this device's person first,
    /// as `"me"` if they wished it, the rest sorted by name.
    pub wished_by: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDetail {
    pub edition_id: String,
    pub work_id: String,
    pub kind: Kind,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    /// Every role but author, ordered by role (`ContributorRole::ALL`) then position.
    pub contributors: Vec<ContributorDetail>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub isbn13: Option<String>,
    pub cover: Option<String>,
    pub google_books_id: Option<String>,
    pub open_library_id: Option<String>,
    pub barcode: Option<String>,
    pub notes: Option<String>,
    pub number: Option<String>,
    pub players_min: Option<i64>,
    pub players_max: Option<i64>,
    /// Boardgame or film, whichever the edition's kind is; never both at once.
    pub duration_minutes: Option<i64>,
    pub min_age: Option<i64>,
    pub seasons: Option<i64>,
    pub copies: Vec<CopyDetail>,
    /// This device may delete the whole book; otherwise it can only ask (D73).
    pub can_delete: bool,
    /// Sorted case-insensitively.
    pub genres: Vec<String>,
    /// Names, sorted case-insensitively.
    pub tags: Vec<String>,
    /// Names of the members who wished the work (D61): this device's person first,
    /// as `"me"` if they wished it, the rest sorted by name.
    pub wished_by: Vec<String>,
    /// The series this work belongs to.
    pub series: Vec<WorkSeries>,
    /// Works this one requires (an expansion needs its base game).
    pub requires: Vec<RelatedWork>,
    /// Works that require this one.
    pub required_by: Vec<RelatedWork>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkSeries {
    pub series_id: String,
    pub name: String,
    pub ordered: bool,
    pub position: Option<i64>,
}

/// A work related to another (D61), with enough to show "you have the expansion
/// but not the base game": its first edition and how many copies it has.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWork {
    pub work_id: String,
    pub edition_id: Option<String>,
    pub title: String,
    pub copies: i64,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributorDetail {
    pub name: String,
    pub role: ContributorRole,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyDetail {
    pub id: String,
    pub owner: Option<String>,
    pub format: Format,
    pub quantity: i64,
    pub location: Option<String>,
    pub acquired_on: Option<String>,
    /// A video game copy's platform: belongs to the copy, not the edition.
    pub platform: Option<String>,
    /// A TV series copy's held seasons, as free text ("1-2", "1-3, 5").
    pub seasons: Option<String>,
    /// This device may delete the copy; otherwise it can only ask (D73).
    pub can_delete: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub operation_id: String,
    pub intent: String,
    /// Milliseconds since the Unix epoch, from the operation's HLC.
    pub timestamp: u64,
}

/// `cover` is the file name of an already stored cover, if any.
pub fn create_book(
    conn: &mut Connection,
    library: &str,
    book: &ValidBook,
    cover: Option<&str>,
) -> Result<CreatedBook> {
    create_book_accepting(conn, library, book, cover, None)
}

/// Adds a book; with `request`, in the same operation that accepts that request,
/// for an administrator who corrected what was asked before accepting it (D73).
pub fn create_book_accepting(
    conn: &mut Connection,
    library: &str,
    book: &ValidBook,
    cover: Option<&str>,
    request: Option<&str>,
) -> Result<CreatedBook> {
    let tx = conn.transaction()?;
    let mut plan = Plan::new(&tx, library);
    if let Some(request) = request {
        crate::requests::ensure_pending(&tx, library, request)?;
        plan.changes.push(Change::create(
            Entity::ChangeRequest,
            request,
            json!({ "status": "accepted" }),
        ));
    }

    let existing_edition_id = match &book.edition.isbn {
        Some(isbn) => edition_with_isbn(&tx, library, isbn.as_str())?,
        None => None,
    }
    .or(match &book.edition.barcode {
        Some(barcode) => edition_with_barcode(&tx, library, barcode)?,
        None => None,
    });
    let existing_edition = existing_edition_id.is_some();
    let edition_id = match existing_edition_id {
        Some(id) => id,
        None => plan.add_edition(&book.edition, cover)?,
    };
    plan.add_copy(&edition_id, &book.copy)?;

    let intent = if request.is_some() {
        "request.accept"
    } else if existing_edition {
        "copy.add"
    } else {
        "book.add"
    };
    plan.record(intent)?;
    tx.commit()?;
    Ok(CreatedBook {
        edition_id,
        existing_edition,
    })
}

/// Adds a work with an edition and no copy (D61): the catalog knows about it
/// ("I'm missing it") without owning it yet. `cover` is the file name of an
/// already stored cover, if any.
pub fn create_missing(
    conn: &mut Connection,
    library: &str,
    edition: &ValidEdition,
    cover: Option<&str>,
) -> Result<CreatedBook> {
    let tx = conn.transaction()?;
    let mut plan = Plan::new(&tx, library);

    let existing_edition_id = match &edition.isbn {
        Some(isbn) => edition_with_isbn(&tx, library, isbn.as_str())?,
        None => None,
    }
    .or(match &edition.barcode {
        Some(barcode) => edition_with_barcode(&tx, library, barcode)?,
        None => None,
    });
    let existing_edition = existing_edition_id.is_some();
    let edition_id = match existing_edition_id {
        Some(id) => id,
        None => plan.add_edition(edition, cover)?,
    };

    // No-op when the edition already existed: `plan.changes` is empty, and
    // `Plan::record` skips recording an operation with nothing in it.
    plan.record("work.add_missing")?;
    tx.commit()?;
    Ok(CreatedBook {
        edition_id,
        existing_edition,
    })
}

pub fn update_edition(
    conn: &mut Connection,
    library: &str,
    edition_id: &str,
    edition: &ValidEdition,
    new_cover: Option<&str>,
) -> Result<()> {
    let tx = conn.transaction()?;
    let work_id = work_of(&tx, library, edition_id)?;

    if let Some(isbn) = &edition.isbn
        && edition_with_isbn(&tx, library, isbn.as_str())?.is_some_and(|other| other != edition_id)
    {
        return Err(Error::Validation(vec![FieldError::new(
            "isbn",
            ErrorCode::IsbnInUse,
        )]));
    }
    if let Some(barcode) = &edition.barcode
        && edition_with_barcode(&tx, library, barcode)?.is_some_and(|other| other != edition_id)
    {
        return Err(Error::Validation(vec![FieldError::new(
            "barcode",
            ErrorCode::BarcodeInUse,
        )]));
    }

    let mut plan = Plan::new(&tx, library);
    let publisher_id = plan.publisher_id(edition)?;
    plan.update_if_changed(
        Entity::Edition,
        edition_id,
        json!({
            "title": edition.title,
            "subtitle": edition.subtitle,
            "publisher_id": publisher_id,
            "published_year": edition.published_year,
            "language": edition.language,
            "barcode": edition.barcode,
            "notes": edition.notes,
        }),
    )?;
    // Without a new cover the current one stays; clearing covers is not supported yet.
    if let Some(cover) = new_cover {
        plan.update_if_changed(Entity::Edition, edition_id, json!({ "cover": cover }))?;
    }

    // A change of kind moves the details from one table to another.
    let current_kind: String = tx.query_row(
        "SELECT kind FROM works WHERE library_id = ?1 AND id = ?2",
        [library, &work_id],
        |row| row.get(0),
    )?;
    // book_details is kept for books only; other kinds keep at most one details row.
    let keep_details = if edition.kind == Kind::Book {
        Some(Entity::BookDetails)
    } else {
        edition.details.entity()
    };
    if current_kind != edition.kind.as_str() {
        plan.update_if_changed(
            Entity::Work,
            &work_id,
            json!({ "kind": edition.kind.as_str() }),
        )?;
        for details in DETAILS {
            if Some(details) != keep_details && exists(&tx, library, details, edition_id)? {
                plan.changes.push(Change::delete(details, edition_id));
            }
        }
    }
    if edition.kind == Kind::Book {
        plan.update_if_changed(
            Entity::BookDetails,
            edition_id,
            book_details_fields(edition),
        )?;
    } else if let Some(entity) = edition.details.entity() {
        if exists(&tx, library, entity, edition_id)? {
            plan.update_if_changed(entity, edition_id, edition.details.fields())?;
        } else {
            plan.changes
                .push(Change::create(entity, edition_id, edition.details.fields()));
        }
    }

    // A work with a single edition takes its title from it.
    let edition_count: i64 = tx.query_row(
        "SELECT count(*) FROM editions WHERE library_id = ?1 AND work_id = ?2",
        [library, &work_id],
        |row| row.get(0),
    )?;
    if edition_count == 1 {
        plan.update_if_changed(Entity::Work, &work_id, json!({ "title": edition.title }))?;
    }
    plan.sync_all_contributors(&work_id, edition)?;
    plan.sync_genres(&work_id, &edition.genres)?;
    plan.sync_tags(&work_id, &edition.tags)?;

    plan.record("edition.edit")?;
    tx.commit()?;
    Ok(())
}

pub fn add_copy(
    conn: &mut Connection,
    library: &str,
    edition_id: &str,
    copy: &ValidCopy,
) -> Result<String> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, library, Entity::Edition, edition_id)?;
    let mut plan = Plan::new(&tx, library);
    let copy_id = plan.add_copy(edition_id, copy)?;
    plan.record("copy.add")?;
    tx.commit()?;
    Ok(copy_id)
}

pub fn update_copy(
    conn: &mut Connection,
    library: &str,
    copy_id: &str,
    copy: &ValidCopy,
) -> Result<()> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, library, Entity::Copy, copy_id)?;
    let mut plan = Plan::new(&tx, library);
    let owner_id = plan.owner_id(copy)?;
    plan.update_if_changed(Entity::Copy, copy_id, copy_fields(copy, owner_id))?;
    plan.record("copy.edit")?;
    tx.commit()?;
    Ok(())
}

pub fn delete_copy(conn: &mut Connection, library: &str, copy_id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, library, Entity::Copy, copy_id)?;
    ops::record(
        &tx,
        library,
        "copy.remove",
        vec![Change::delete(Entity::Copy, copy_id)],
    )?;
    tx.commit()?;
    Ok(())
}

/// Removes the edition with its copies, and the work too if no other edition remains.
/// People and members stay: they are catalogues of their own.
pub fn delete_book(conn: &mut Connection, library: &str, edition_id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    delete_book_in(&tx, library, edition_id, "book.remove")?;
    tx.commit()?;
    Ok(())
}

/// Moves a book to another library (D46): it is created there with new ids, since
/// the tombstones left in the origin must not stop it from coming back one day,
/// and removed from the origin, where its history stays. An edition with the same
/// ISBN in the destination gets the copies instead. Returns the new edition id.
pub fn move_book(conn: &mut Connection, from: &str, edition_id: &str, to: &str) -> Result<String> {
    let book = get_book(conn, from, edition_id)?;
    let edition = EditionInput {
        title: book.title,
        subtitle: book.subtitle,
        authors: book.authors,
        publisher: book.publisher,
        published_year: book.published_year,
        language: book.language,
        isbn: book.isbn13,
        cover_url: None,
        google_books_id: book.google_books_id,
        open_library_id: book.open_library_id,
        kind: book.kind,
        contributors: book
            .contributors
            .into_iter()
            .map(|c| ContributorInput {
                name: c.name,
                role: c.role,
            })
            .collect(),
        barcode: book.barcode,
        notes: book.notes,
        number: book.number,
        players_min: book.players_min,
        players_max: book.players_max,
        duration_minutes: book.duration_minutes,
        min_age: book.min_age,
        seasons: book.seasons,
        genres: book.genres,
        tags: book.tags,
    }
    .validate()
    .map_err(Error::Validation)?;
    let copies = book
        .copies
        .into_iter()
        .map(|copy| {
            CopyInput {
                owner: copy.owner,
                format: copy.format,
                quantity: u32::try_from(copy.quantity).unwrap_or_default(),
                location: copy.location,
                acquired_on: copy.acquired_on,
                platform: copy.platform,
                seasons: copy.seasons,
            }
            .validate()
            .map_err(Error::Validation)
        })
        .collect::<Result<Vec<_>>>()?;

    let tx = conn.transaction()?;
    let mut plan = Plan::new(&tx, to);
    let new_edition = match &edition.isbn {
        Some(isbn) => edition_with_isbn(&tx, to, isbn.as_str())?,
        None => None,
    };
    let new_edition = match new_edition {
        Some(id) => id,
        None => plan.add_edition(&edition, book.cover.as_deref())?,
    };
    for copy in &copies {
        plan.add_copy(&new_edition, copy)?;
    }
    plan.record("book.move_in")?;
    delete_book_in(&tx, from, edition_id, "book.move_out")?;
    tx.commit()?;
    Ok(new_edition)
}

fn delete_book_in(
    tx: &Transaction<'_>,
    library: &str,
    edition_id: &str,
    intent: &str,
) -> Result<()> {
    let changes = book_removal(tx, library, edition_id)?;
    ops::record(tx, library, intent, changes)?;
    Ok(())
}

/// The changes that remove an edition with its copies, and its work if no other
/// edition remains.
fn book_removal(conn: &Connection, library: &str, edition_id: &str) -> Result<Vec<Change>> {
    let work_id = work_of(conn, library, edition_id)?;

    // Children before parents, so every change finds its row.
    let mut changes: Vec<Change> = ids(
        conn,
        "SELECT id FROM copies WHERE library_id = ?1 AND edition_id = ?2",
        library,
        edition_id,
    )?
    .into_iter()
    .map(|id| Change::delete(Entity::Copy, id))
    .collect();
    for details in DETAILS {
        if exists(conn, library, details, edition_id)? {
            changes.push(Change::delete(details, edition_id));
        }
    }
    changes.push(Change::delete(Entity::Edition, edition_id));

    let other_editions: i64 = conn.query_row(
        "SELECT count(*) FROM editions WHERE library_id = ?1 AND work_id = ?2 AND id <> ?3",
        [library, &work_id, edition_id],
        |row| row.get(0),
    )?;
    if other_editions == 0 {
        changes.extend(
            ids(
                conn,
                "SELECT id FROM work_contributors WHERE library_id = ?1 AND work_id = ?2",
                library,
                &work_id,
            )?
            .into_iter()
            .map(|id| Change::delete(Entity::WorkContributor, id)),
        );
        changes.extend(
            ids(
                conn,
                "SELECT id FROM work_tags WHERE library_id = ?1 AND work_id = ?2",
                library,
                &work_id,
            )?
            .into_iter()
            .map(|id| Change::delete(Entity::WorkTag, id)),
        );
        changes.extend(
            ids(
                conn,
                "SELECT id FROM work_genres WHERE library_id = ?1 AND work_id = ?2",
                library,
                &work_id,
            )?
            .into_iter()
            .map(|id| Change::delete(Entity::WorkGenre, id)),
        );
        changes.extend(
            ids(
                conn,
                "SELECT id FROM series_entries WHERE library_id = ?1 AND work_id = ?2",
                library,
                &work_id,
            )?
            .into_iter()
            .map(|id| Change::delete(Entity::SeriesEntry, id)),
        );
        changes.extend(
            ids(
                conn,
                "SELECT id FROM work_relations WHERE library_id = ?1
                  AND (work_id = ?2 OR related_work_id = ?2)",
                library,
                &work_id,
            )?
            .into_iter()
            .map(|id| Change::delete(Entity::WorkRelation, id)),
        );
        changes.push(Change::delete(Entity::Work, &work_id));
    }
    Ok(changes)
}

/// Collects the changes of one operation, reusing rows that already exist.
struct Plan<'a> {
    tx: &'a Transaction<'a>,
    library: &'a str,
    changes: Vec<Change>,
    /// Rows created earlier in this same plan, so a repeated name is created once.
    planned: HashMap<(Entity, String), String>,
}

impl<'a> Plan<'a> {
    fn new(tx: &'a Transaction<'a>, library: &'a str) -> Self {
        Self {
            tx,
            library,
            changes: Vec::new(),
            planned: HashMap::new(),
        }
    }

    /// Records the collected changes as one operation; does nothing if there are none.
    fn record(self, intent: &str) -> Result<()> {
        if !self.changes.is_empty() {
            ops::record(self.tx, self.library, intent, self.changes)?;
        }
        Ok(())
    }

    fn add_edition(&mut self, edition: &ValidEdition, cover: Option<&str>) -> Result<String> {
        let work_id = new_id();
        self.changes.push(Change::create(
            Entity::Work,
            &work_id,
            json!({ "kind": edition.kind.as_str(), "title": edition.title }),
        ));
        self.sync_all_contributors(&work_id, edition)?;
        self.sync_genres(&work_id, &edition.genres)?;
        self.sync_tags(&work_id, &edition.tags)?;
        let publisher_id = self.publisher_id(edition)?;

        let edition_id = new_id();
        self.changes.push(Change::create(
            Entity::Edition,
            &edition_id,
            json!({
                "work_id": work_id,
                "title": edition.title,
                "subtitle": edition.subtitle,
                "publisher_id": publisher_id,
                "published_year": edition.published_year,
                "language": edition.language,
                "cover": cover,
                "barcode": edition.barcode,
                "notes": edition.notes,
            }),
        ));
        if edition.kind == Kind::Book {
            self.changes.push(Change::create(
                Entity::BookDetails,
                &edition_id,
                book_details_fields(edition),
            ));
        } else if let Some(entity) = edition.details.entity() {
            self.changes.push(Change::create(
                entity,
                &edition_id,
                edition.details.fields(),
            ));
        }
        Ok(edition_id)
    }

    fn add_copy(&mut self, edition_id: &str, copy: &ValidCopy) -> Result<String> {
        let owner_id = self.owner_id(copy)?;
        let mut fields = copy_fields(copy, owner_id);
        fields["edition_id"] = json!(edition_id);
        let copy_id = new_id();
        self.changes
            .push(Change::create(Entity::Copy, &copy_id, fields));
        Ok(copy_id)
    }

    fn publisher_id(&mut self, edition: &ValidEdition) -> Result<Option<String>> {
        edition
            .publisher
            .as_deref()
            .map(|name| self.find_or_create(Entity::Publisher, name))
            .transpose()
    }

    fn owner_id(&mut self, copy: &ValidCopy) -> Result<Option<String>> {
        copy.owner
            .as_deref()
            .map(|name| self.find_or_create(Entity::Member, name))
            .transpose()
    }

    /// Makes every contributor role of the work match `edition`, in order: adds,
    /// moves and removes rows as needed. An empty list for a role removes its rows,
    /// so every role is synced even when the UI still sends only `authors`.
    fn sync_all_contributors(&mut self, work_id: &str, edition: &ValidEdition) -> Result<()> {
        self.sync_contributors(work_id, ContributorRole::Author, &edition.authors)?;
        for role in ContributorRole::ALL {
            if role == ContributorRole::Author {
                continue;
            }
            let names: Vec<String> = edition
                .contributors
                .iter()
                .filter(|c| c.role == role)
                .map(|c| c.name.clone())
                .collect();
            self.sync_contributors(work_id, role, &names)?;
        }
        Ok(())
    }

    /// Makes the work's contributors in `role` match `names`, in order: adds, moves
    /// and removes rows as needed. Works for new works too (nothing to compare against).
    fn sync_contributors(
        &mut self,
        work_id: &str,
        role: ContributorRole,
        names: &[String],
    ) -> Result<()> {
        let mut current: HashMap<String, (String, i64)> = HashMap::new();
        {
            let mut statement = self.tx.prepare(
                "SELECT person_id, id, position FROM work_contributors
                  WHERE library_id = ?1 AND work_id = ?2 AND role = ?3",
            )?;
            let rows = statement.query_map([self.library, work_id, role.as_str()], |row| {
                Ok((row.get(0)?, (row.get(1)?, row.get(2)?)))
            })?;
            for row in rows {
                let (person_id, entry) = row?;
                current.insert(person_id, entry);
            }
        }

        for (position, name) in names.iter().enumerate() {
            let position = i64::try_from(position).unwrap_or(i64::MAX);
            let person_id = self.find_or_create(Entity::Person, name)?;
            match current.remove(&person_id) {
                Some((_, old_position)) if old_position == position => {}
                Some((contributor_id, _)) => self.changes.push(Change::update(
                    Entity::WorkContributor,
                    contributor_id,
                    json!({ "position": position }),
                )),
                None => self.changes.push(Change::create(
                    Entity::WorkContributor,
                    new_id(),
                    json!({ "work_id": work_id, "person_id": person_id, "role": role.as_str(), "position": position }),
                )),
            }
        }
        for (contributor_id, _) in current.into_values() {
            self.changes
                .push(Change::delete(Entity::WorkContributor, contributor_id));
        }
        Ok(())
    }

    /// Makes the work's genres match `genres`: creates missing links, deletes every
    /// row of a genre no longer wanted. Genres kept are left as they are, duplicates
    /// included: two devices may have added the same one offline (no unique
    /// constraint), and both rows must survive for the devices to converge.
    fn sync_genres(&mut self, work_id: &str, genres: &[String]) -> Result<()> {
        let mut current: HashMap<String, Vec<String>> = HashMap::new();
        {
            let mut statement = self.tx.prepare(
                "SELECT genre, id FROM work_genres WHERE library_id = ?1 AND work_id = ?2",
            )?;
            let rows = statement.query_map([self.library, work_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (genre, id) = row?;
                current.entry(genre).or_default().push(id);
            }
        }
        for genre in genres {
            if !current.contains_key(genre) {
                self.changes.push(Change::create(
                    Entity::WorkGenre,
                    new_id(),
                    json!({ "work_id": work_id, "genre": genre }),
                ));
            }
        }
        for (genre, ids) in current {
            if !genres.contains(&genre) {
                for id in ids {
                    self.changes.push(Change::delete(Entity::WorkGenre, id));
                }
            }
        }
        Ok(())
    }

    /// Makes the work's tags match `names` (found or created by name): same rule as
    /// `sync_genres` for rows kept.
    fn sync_tags(&mut self, work_id: &str, names: &[String]) -> Result<()> {
        let mut desired = Vec::with_capacity(names.len());
        for name in names {
            desired.push(self.find_or_create(Entity::Tag, name)?);
        }

        let mut current: HashMap<String, Vec<String>> = HashMap::new();
        {
            let mut statement = self.tx.prepare(
                "SELECT tag_id, id FROM work_tags WHERE library_id = ?1 AND work_id = ?2",
            )?;
            let rows = statement.query_map([self.library, work_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (tag_id, id) = row?;
                current.entry(tag_id).or_default().push(id);
            }
        }
        for tag_id in &desired {
            if !current.contains_key(tag_id) {
                self.changes.push(Change::create(
                    Entity::WorkTag,
                    new_id(),
                    json!({ "work_id": work_id, "tag_id": tag_id }),
                ));
            }
        }
        for (tag_id, ids) in current {
            if !desired.contains(&tag_id) {
                for id in ids {
                    self.changes.push(Change::delete(Entity::WorkTag, id));
                }
            }
        }
        Ok(())
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
                &format!(
                    "SELECT id FROM {table} WHERE library_id = ?1 AND name = ?2 COLLATE NOCASE
                      ORDER BY id LIMIT 1"
                ),
                [self.library, name],
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

    /// Adds an update with only the fields whose value differs from the stored row.
    /// Fewer fields per change means fewer conflicts when devices sync (D13).
    fn update_if_changed(&mut self, entity: Entity, id: &str, desired: Value) -> Result<()> {
        let Value::Object(desired) = desired else {
            return Ok(());
        };
        let columns: Vec<&str> = desired
            .keys()
            .filter_map(|name| entity.field(name).map(|spec| spec.name))
            .collect();
        let current: Vec<Value> = self
            .tx
            .query_row(
                &format!(
                    "SELECT {} FROM {} WHERE {} = ?1 AND {} = ?2",
                    columns.join(", "),
                    entity.table(),
                    entity.library_column(),
                    entity.id_column()
                ),
                [self.library, id],
                |row| {
                    (0..columns.len())
                        .map(|i| row.get_ref(i).map(to_json))
                        .collect()
                },
            )
            .optional()?
            .ok_or_else(|| missing(entity, id))?;

        let changed: Map<String, Value> = columns
            .iter()
            .zip(current)
            .filter(|(column, old)| desired[**column] != *old)
            .map(|(column, _)| (column.to_string(), desired[*column].clone()))
            .collect();
        if !changed.is_empty() {
            self.changes
                .push(Change::update(entity, id, Value::Object(changed)));
        }
        Ok(())
    }
}

fn copy_fields(copy: &ValidCopy, owner_id: Option<String>) -> Value {
    json!({
        "owner_id": owner_id,
        "format": copy.format.as_str(),
        "quantity": copy.quantity,
        "location": copy.location,
        "acquired_on": copy.acquired_on.map(|date| date.to_string()),
        "platform": copy.platform,
        "seasons": copy.seasons,
    })
}

fn to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => json!(String::from_utf8_lossy(bytes)),
    }
}

fn book_details_fields(edition: &ValidEdition) -> Value {
    json!({
        "isbn13": edition.isbn.as_ref().map(|isbn| isbn.as_str()),
        "google_books_id": edition.google_books_id,
        "open_library_id": edition.open_library_id,
    })
}

pub fn edition_with_isbn(conn: &Connection, library: &str, isbn13: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT edition_id FROM book_details WHERE library_id = ?1 AND isbn13 = ?2
              ORDER BY edition_id LIMIT 1",
            [library, isbn13],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn edition_with_barcode(
    conn: &Connection,
    library: &str,
    barcode: &str,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM editions WHERE library_id = ?1 AND barcode = ?2 ORDER BY id LIMIT 1",
            [library, barcode],
            |row| row.get(0),
        )
        .optional()?)
}

/// Which existing edition, if any, a search candidate already matches: by ISBN first
/// (parsed, so "978-..." and "9780..." are the same), then by barcode.
pub fn edition_matching_candidate(
    conn: &Connection,
    library: &str,
    isbn: Option<&str>,
    barcode: Option<&str>,
) -> Result<Option<String>> {
    if let Some(isbn) = isbn.and_then(|isbn| Isbn::parse(isbn).ok()) {
        let found = edition_with_isbn(conn, library, isbn.as_str())?;
        if found.is_some() {
            return Ok(found);
        }
    }
    match barcode {
        Some(barcode) => edition_with_barcode(conn, library, barcode),
        None => Ok(None),
    }
}

fn work_of(conn: &Connection, library: &str, edition_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT work_id FROM editions WHERE library_id = ?1 AND id = ?2",
        [library, edition_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| missing(Entity::Edition, edition_id))
}

pub(crate) fn exists(conn: &Connection, library: &str, entity: Entity, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT 1 FROM {} WHERE {} = ?1 AND {} = ?2",
                entity.table(),
                entity.library_column(),
                entity.id_column()
            ),
            [library, id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(crate) fn ensure_exists(
    conn: &Connection,
    library: &str,
    entity: Entity,
    id: &str,
) -> Result<()> {
    if exists(conn, library, entity, id)? {
        Ok(())
    } else {
        Err(missing(entity, id))
    }
}

pub(crate) fn missing(entity: Entity, id: &str) -> Error {
    Error::MissingRow {
        entity: entity.table(),
        id: id.to_owned(),
    }
}

pub(crate) fn ids(conn: &Connection, sql: &str, library: &str, param: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([library, param], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Whether a work's edition has copies (D61): `Missing` is what we catalog
/// without owning yet, like the next volume of a saga we lack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Holding {
    Owned,
    Missing,
}

/// Search text and copy filters; empty means "everything".
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BookFilter {
    pub query: Option<String>,
    pub kind: Option<Kind>,
    pub format: Option<Format>,
    /// `Owned` = the edition has copies, `Missing` = it has none (D61);
    /// `None` matches everything.
    pub holding: Option<Holding>,
    /// Owner name: duplicates of the same member created on other devices match too (D33).
    pub owner: Option<String>,
    pub location: Option<String>,
    /// My status, any of them; `none` matches works I have not marked (D68).
    pub my_statuses: Vec<String>,
    pub my_favorites: bool,
    /// Works I wish I had (D61).
    pub my_wished: bool,
    /// Works someone other than me wishes.
    pub others_wished: bool,
    /// A work matches if it has any of these (ignored when empty).
    pub genres: Vec<String>,
    /// Tag ids; a work matches if it has any of these (ignored when empty).
    pub tags: Vec<String>,
}

pub fn list_books(
    conn: &Connection,
    library: &str,
    filter: &BookFilter,
) -> Result<Vec<BookSummary>> {
    // The first argument is this device, whose person's marks come with each work.
    let device = ops::device_id(conn)?;
    let my_member: Option<String> = conn
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, &device],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let mut conditions: Vec<String> = vec!["e.library_id = ?".to_owned()];
    let mut args: Vec<SqlValue> = vec![
        my_member.clone().map_or(SqlValue::Null, SqlValue::Text),
        my_member.clone().map_or(SqlValue::Null, SqlValue::Text),
        SqlValue::Text(device.clone()),
        SqlValue::Text(library.to_owned()),
    ];

    if let Some(query) = filter.query.as_deref().and_then(search::fts_query) {
        conditions.push(
            "e.id IN (SELECT edition_id FROM search_index
                       WHERE search_index MATCH ? AND library_id = e.library_id)"
                .to_owned(),
        );
        args.push(SqlValue::Text(query));
    }
    if let Some(kind) = filter.kind {
        conditions.push("w.kind = ?".to_owned());
        args.push(SqlValue::Text(kind.as_str().to_owned()));
    }
    if let Some(holding) = filter.holding {
        let cmp = match holding {
            Holding::Owned => ">",
            Holding::Missing => "=",
        };
        conditions.push(format!(
            "(SELECT coalesce(sum(c.quantity), 0) FROM copies c
               WHERE c.library_id = e.library_id AND c.edition_id = e.id) {cmp} 0"
        ));
    }
    // One copy must match every copy filter at once.
    if filter.format.is_some() || filter.owner.is_some() || filter.location.is_some() {
        conditions.push(
            "EXISTS (SELECT 1 FROM copies c WHERE c.library_id = e.library_id AND c.edition_id = e.id
                      AND (? IS NULL OR c.format = ?)
                      AND (? IS NULL OR c.owner_id IN (SELECT id FROM members
                                                         WHERE library_id = e.library_id
                                                           AND name = ? COLLATE NOCASE))
                      AND (? IS NULL OR c.location = ?))"
                .to_owned(),
        );
        let text =
            |value: Option<&str>| value.map_or(SqlValue::Null, |v| SqlValue::Text(v.to_owned()));
        for value in [
            text(filter.format.map(Format::as_str)),
            text(filter.owner.as_deref()),
            text(filter.location.as_deref()),
        ] {
            args.push(value.clone());
            args.push(value);
        }
    }
    if !filter.my_statuses.is_empty() {
        let set = filter.my_statuses.iter().filter(|s| *s != "none");
        let mut any: Vec<String> = Vec::new();
        let placeholders: Vec<&str> = set.clone().map(|_| "?").collect();
        if !placeholders.is_empty() {
            any.push(format!("mk.status IN ({})", placeholders.join(", ")));
            args.extend(set.map(|s| SqlValue::Text(s.clone())));
        }
        if filter.my_statuses.iter().any(|s| s == "none") {
            any.push("mk.status IS NULL".to_owned());
        }
        conditions.push(format!("({})", any.join(" OR ")));
    }
    if filter.my_favorites {
        conditions.push("mk.favorite = 1".to_owned());
    }
    if filter.my_wished {
        conditions.push("mk.wished = 1".to_owned());
    }
    if filter.others_wished {
        // `IS NOT` on purpose: a device with no person sees everyone's wishes as "others".
        conditions.push(
            "EXISTS (SELECT 1 FROM marks o WHERE o.library_id = e.library_id
                       AND o.work_id = e.work_id AND o.wished = 1
                       AND o.member_id IS NOT (SELECT member_id FROM devices
                                                 WHERE library_id = e.library_id AND id = ?))"
                .to_owned(),
        );
        args.push(SqlValue::Text(device.clone()));
    }
    if !filter.genres.is_empty() {
        let placeholders: Vec<&str> = filter.genres.iter().map(|_| "?").collect();
        conditions.push(format!(
            "EXISTS (SELECT 1 FROM work_genres wg WHERE wg.library_id = e.library_id
                       AND wg.work_id = e.work_id AND wg.genre IN ({}))",
            placeholders.join(", ")
        ));
        args.extend(filter.genres.iter().map(|g| SqlValue::Text(g.clone())));
    }
    if !filter.tags.is_empty() {
        let placeholders: Vec<&str> = filter.tags.iter().map(|_| "?").collect();
        conditions.push(format!(
            "EXISTS (SELECT 1 FROM work_tags wt WHERE wt.library_id = e.library_id
                       AND wt.work_id = e.work_id AND wt.tag_id IN ({}))",
            placeholders.join(", ")
        ));
        args.extend(filter.tags.iter().map(|t| SqlValue::Text(t.clone())));
    }
    let where_clause = format!("WHERE {}", conditions.join(" AND "));

    let mut statement = conn.prepare(&format!(
        "SELECT e.id, e.library_id, e.title, e.subtitle, pu.name, e.published_year, bd.isbn13, e.cover,
            (SELECT group_concat(p.name, char(31) ORDER BY wc.position)
               FROM work_contributors wc
               JOIN people p ON p.library_id = wc.library_id AND p.id = wc.person_id
              WHERE wc.library_id = e.library_id AND wc.work_id = e.work_id AND wc.role = 'author'),
            (SELECT coalesce(sum(c.quantity), 0) FROM copies c
              WHERE c.library_id = e.library_id AND c.edition_id = e.id),
            (SELECT group_concat(format, char(31))
               FROM (SELECT DISTINCT c.format FROM copies c
                      WHERE c.library_id = e.library_id AND c.edition_id = e.id)),
            (SELECT group_concat(name, char(31) ORDER BY name COLLATE NOCASE)
               FROM (SELECT DISTINCT m.name FROM copies c
                       JOIN members m ON m.library_id = c.library_id AND m.id = c.owner_id
                      WHERE c.library_id = e.library_id AND c.edition_id = e.id)),
            mk.status, mk.favorite, mk.wished, w.kind,
            (SELECT group_concat(name, char(31) ORDER BY name COLLATE NOCASE)
               FROM (SELECT DISTINCT t.name AS name FROM work_tags wt
                       JOIN tags t ON t.library_id = wt.library_id AND t.id = wt.tag_id
                      WHERE wt.library_id = e.library_id AND wt.work_id = e.work_id)),
            (SELECT group_concat(
                      CASE WHEN wm.member_id = ? THEN 'me' ELSE wmem.name END,
                      char(31) ORDER BY wm.member_id = ? DESC, wmem.name COLLATE NOCASE
                    )
               FROM marks wm JOIN members wmem ON wmem.library_id = wm.library_id AND wmem.id = wm.member_id
              WHERE wm.library_id = e.library_id AND wm.work_id = w.id AND wm.wished = 1)
         FROM editions e
         JOIN works w ON w.library_id = e.library_id AND w.id = e.work_id
         LEFT JOIN marks mk ON mk.library_id = e.library_id AND mk.work_id = e.work_id
              AND mk.member_id = (SELECT member_id FROM devices
                                   WHERE library_id = e.library_id AND id = ?)
         LEFT JOIN book_details bd ON bd.library_id = e.library_id AND bd.edition_id = e.id
         LEFT JOIN publishers pu ON pu.library_id = e.library_id AND pu.id = e.publisher_id
         {where_clause}
         ORDER BY e.title COLLATE NOCASE, e.id"
    ))?;
    let rows = statement.query_map(params_from_iter(args), |row| {
        Ok(BookSummary {
            edition_id: row.get(0)?,
            library_id: row.get(1)?,
            title: row.get(2)?,
            subtitle: row.get(3)?,
            publisher: row.get(4)?,
            published_year: row.get(5)?,
            isbn13: row.get(6)?,
            cover: row.get(7)?,
            authors: split_list(row.get(8)?),
            copy_count: row.get(9)?,
            formats: split_list(row.get(10)?)
                .iter()
                .filter_map(|format| format.parse().ok())
                .collect(),
            owners: split_list(row.get(11)?),
            my_status: row.get(12)?,
            my_favorite: row.get::<_, Option<i64>>(13)?.unwrap_or(0) != 0,
            my_wished: row.get::<_, Option<i64>>(14)?.unwrap_or(0) != 0,
            kind: row.get::<_, String>(15)?.parse().unwrap_or_default(),
            tags: split_list(row.get(16)?),
            wished_by: split_list(row.get(17)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Books of every library on the device, library by library.
pub fn list_books_everywhere(conn: &Connection, filter: &BookFilter) -> Result<Vec<BookSummary>> {
    let libraries: Vec<String> = conn
        .prepare("SELECT id FROM libraries ORDER BY name COLLATE NOCASE, id")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut books = Vec::new();
    for library in libraries {
        books.extend(list_books(conn, &library, filter)?);
    }
    Ok(books)
}

/// What the catalog knows about an ISBN, from the local database only, so it works
/// in a shop without coverage.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsbnCheck {
    pub isbn13: String,
    pub books: Vec<BookSummary>,
}

/// Looks in every library on the device: "do I have it?" is about the household, not one list.
pub fn check_isbn(conn: &Connection, raw: &str) -> Result<IsbnCheck> {
    let isbn = Isbn::parse(raw)
        .map_err(|error| Error::Validation(vec![FieldError::new("isbn", error.into())]))?;
    let filter = BookFilter {
        query: Some(isbn.as_str().to_owned()),
        ..BookFilter::default()
    };
    Ok(IsbnCheck {
        isbn13: isbn.as_str().to_owned(),
        books: list_books_everywhere(conn, &filter)?,
    })
}

/// Values already in the catalog, for autocompletion and filters.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogOptions {
    pub authors: Vec<String>,
    pub publishers: Vec<String>,
    pub owners: Vec<String>,
    pub locations: Vec<String>,
}

pub fn catalog_options(conn: &Connection, library: &str) -> Result<CatalogOptions> {
    let names = |sql: &str| -> Result<Vec<String>> {
        Ok(conn
            .prepare(sql)?
            .query_map([library], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    };
    // Each name once, even if devices created it separately (D33).
    let unique = |table: &str| {
        names(&format!(
            "SELECT min(name) FROM {table} WHERE library_id = ?1
              GROUP BY name COLLATE NOCASE ORDER BY 1 COLLATE NOCASE"
        ))
    };
    Ok(CatalogOptions {
        authors: unique("people")?,
        publishers: unique("publishers")?,
        owners: unique("members")?,
        locations: names(
            "SELECT DISTINCT location FROM copies WHERE library_id = ?1 AND location IS NOT NULL
              ORDER BY location COLLATE NOCASE",
        )?,
    })
}

pub fn get_book(conn: &Connection, library: &str, edition_id: &str) -> Result<BookDetail> {
    let mut book = conn
        .query_row(
            "SELECT e.id, e.work_id, e.title, e.subtitle, pu.name, e.published_year, e.language,
                    bd.isbn13, e.cover, bd.google_books_id, bd.open_library_id,
                    (SELECT group_concat(p.name, char(31) ORDER BY wc.position)
                       FROM work_contributors wc
                       JOIN people p ON p.library_id = wc.library_id AND p.id = wc.person_id
                      WHERE wc.library_id = e.library_id AND wc.work_id = e.work_id
                        AND wc.role = 'author'),
                    w.kind, e.barcode, cd.number,
                    gd.players_min, gd.players_max, coalesce(gd.duration_minutes, fd.duration_minutes),
                    gd.min_age, td.seasons, e.notes
               FROM editions e
               JOIN works w ON w.library_id = e.library_id AND w.id = e.work_id
               LEFT JOIN book_details bd ON bd.library_id = e.library_id AND bd.edition_id = e.id
               LEFT JOIN publishers pu ON pu.library_id = e.library_id AND pu.id = e.publisher_id
               LEFT JOIN comic_details cd ON cd.library_id = e.library_id AND cd.edition_id = e.id
               LEFT JOIN boardgame_details gd ON gd.library_id = e.library_id AND gd.edition_id = e.id
               LEFT JOIN film_details fd ON fd.library_id = e.library_id AND fd.edition_id = e.id
               LEFT JOIN tv_series_details td ON td.library_id = e.library_id AND td.edition_id = e.id
              WHERE e.library_id = ?1 AND e.id = ?2",
            [library, edition_id],
            |row| {
                Ok(BookDetail {
                    edition_id: row.get(0)?,
                    work_id: row.get(1)?,
                    title: row.get(2)?,
                    subtitle: row.get(3)?,
                    publisher: row.get(4)?,
                    published_year: row.get(5)?,
                    language: row.get(6)?,
                    isbn13: row.get(7)?,
                    cover: row.get(8)?,
                    google_books_id: row.get(9)?,
                    open_library_id: row.get(10)?,
                    authors: split_list(row.get(11)?),
                    kind: row.get::<_, String>(12)?.parse().unwrap_or_default(),
                    barcode: row.get(13)?,
                    contributors: Vec::new(),
                    number: row.get(14)?,
                    players_min: row.get(15)?,
                    players_max: row.get(16)?,
                    duration_minutes: row.get(17)?,
                    min_age: row.get(18)?,
                    seasons: row.get(19)?,
                    notes: row.get(20)?,
                    copies: Vec::new(),
                    can_delete: false,
                    genres: Vec::new(),
                    tags: Vec::new(),
                    wished_by: Vec::new(),
                    series: Vec::new(),
                    requires: Vec::new(),
                    required_by: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| missing(Entity::Edition, edition_id))?;

    let mut contributors_statement = conn.prepare(
        "SELECT p.name, wc.role
           FROM work_contributors wc
           JOIN people p ON p.library_id = wc.library_id AND p.id = wc.person_id
          WHERE wc.library_id = ?1 AND wc.work_id = ?2 AND wc.role <> 'author'
          ORDER BY wc.position",
    )?;
    let mut contributors: Vec<ContributorDetail> = contributors_statement
        .query_map([library, &book.work_id], |row| {
            let role: String = row.get(1)?;
            Ok(ContributorDetail {
                name: row.get(0)?,
                role: role.parse().unwrap_or(ContributorRole::Author),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    // Stable sort: keeps the position order already fetched within each role.
    contributors.sort_by_key(|c| ContributorRole::ALL.iter().position(|r| *r == c.role));
    book.contributors = contributors;

    let mut statement = conn.prepare(
        "SELECT c.id, m.name, c.format, c.quantity, c.location, c.acquired_on, c.platform, c.seasons
           FROM copies c LEFT JOIN members m ON m.library_id = c.library_id AND m.id = c.owner_id
          WHERE c.library_id = ?1 AND c.edition_id = ?2
          ORDER BY c.created_at, c.id",
    )?;
    let rows = statement.query_map([library, edition_id], |row| {
        let format: String = row.get(2)?;
        Ok(CopyDetail {
            id: row.get(0)?,
            owner: row.get(1)?,
            format: format.parse().unwrap_or(Format::Other),
            quantity: row.get(3)?,
            location: row.get(4)?,
            acquired_on: row.get(5)?,
            platform: row.get(6)?,
            seasons: row.get(7)?,
            can_delete: false,
        })
    })?;
    book.copies = rows.collect::<rusqlite::Result<_>>()?;
    for copy in &mut book.copies {
        copy.can_delete =
            ops::would_allow(conn, library, vec![Change::delete(Entity::Copy, &copy.id)])?;
    }
    book.can_delete = ops::would_allow(conn, library, book_removal(conn, library, edition_id)?)?;

    book.genres = conn
        .prepare(
            "SELECT DISTINCT genre FROM work_genres WHERE library_id = ?1 AND work_id = ?2
              ORDER BY genre COLLATE NOCASE",
        )?
        .query_map([library, &book.work_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    book.tags = conn
        .prepare(
            "SELECT DISTINCT t.name FROM work_tags wt
               JOIN tags t ON t.library_id = wt.library_id AND t.id = wt.tag_id
              WHERE wt.library_id = ?1 AND wt.work_id = ?2
              ORDER BY t.name COLLATE NOCASE",
        )?
        .query_map([library, &book.work_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    let device = ops::device_id(conn)?;
    let my_member: Option<String> = conn
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, &device],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    book.wished_by = split_list(conn.query_row(
        "SELECT group_concat(
                      CASE WHEN m.id = ?1 THEN 'me' ELSE m.name END,
                      char(31) ORDER BY m.id = ?1 DESC, m.name COLLATE NOCASE
                    )
               FROM marks mk JOIN members m ON m.library_id = mk.library_id AND m.id = mk.member_id
              WHERE mk.library_id = ?2 AND mk.work_id = ?3 AND mk.wished = 1",
        rusqlite::params![my_member, library, &book.work_id],
        |row| row.get(0),
    )?);

    book.series = conn
        .prepare(
            "SELECT DISTINCT s.id, s.name, s.ordered,
                    (SELECT se2.position FROM series_entries se2
                      WHERE se2.library_id = s.library_id AND se2.series_id = s.id
                        AND se2.work_id = ?2
                      ORDER BY se2.position IS NULL, se2.position LIMIT 1)
               FROM series_entries se
               JOIN series s ON s.library_id = se.library_id AND s.id = se.series_id
              WHERE se.library_id = ?1 AND se.work_id = ?2
              ORDER BY s.name COLLATE NOCASE, s.id",
        )?
        .query_map([library, &book.work_id], |row| {
            Ok(WorkSeries {
                series_id: row.get(0)?,
                name: row.get(1)?,
                ordered: row.get::<_, i64>(2)? != 0,
                position: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let requires_ids = ids(
        conn,
        "SELECT DISTINCT related_work_id FROM work_relations
          WHERE library_id = ?1 AND work_id = ?2 AND relation = 'requires'",
        library,
        &book.work_id,
    )?;
    book.requires = related_works(conn, library, &requires_ids)?;
    let required_by_ids = ids(
        conn,
        "SELECT DISTINCT work_id FROM work_relations
          WHERE library_id = ?1 AND related_work_id = ?2 AND relation = 'requires'",
        library,
        &book.work_id,
    )?;
    book.required_by = related_works(conn, library, &required_by_ids)?;
    Ok(book)
}

/// Each related work with its first edition (by title) and its total copies, so
/// the UI can say "you have the expansion but not the base game", sorted by title.
fn related_works(
    conn: &Connection,
    library: &str,
    work_ids: &[String],
) -> Result<Vec<RelatedWork>> {
    let mut result = Vec::with_capacity(work_ids.len());
    for work_id in work_ids {
        let title: String = conn.query_row(
            "SELECT title FROM works WHERE library_id = ?1 AND id = ?2",
            [library, work_id],
            |row| row.get(0),
        )?;
        let edition_id: Option<String> = conn
            .query_row(
                "SELECT id FROM editions WHERE library_id = ?1 AND work_id = ?2
                  ORDER BY title COLLATE NOCASE, id LIMIT 1",
                [library, work_id],
                |row| row.get(0),
            )
            .optional()?;
        let copies: i64 = conn.query_row(
            "SELECT coalesce(sum(c.quantity), 0) FROM copies c
               JOIN editions e ON e.library_id = c.library_id AND e.id = c.edition_id
              WHERE e.library_id = ?1 AND e.work_id = ?2",
            [library, work_id],
            |row| row.get(0),
        )?;
        result.push(RelatedWork {
            work_id: work_id.clone(),
            edition_id,
            title,
            copies,
        });
    }
    result.sort_by_key(|w| w.title.to_lowercase());
    Ok(result)
}

/// Operations that touched the edition, its work or any of its copies, oldest first.
pub fn book_history(
    conn: &Connection,
    library: &str,
    edition_id: &str,
) -> Result<Vec<HistoryEntry>> {
    let mut statement = conn.prepare(
        "WITH related (id) AS (
             SELECT ?2
             UNION SELECT work_id FROM editions WHERE library_id = ?1 AND id = ?2
             UNION SELECT id FROM copies WHERE library_id = ?1 AND edition_id = ?2
         )
         SELECT DISTINCT o.id, o.intent, o.hlc
           FROM operations o, json_each(o.changes) AS change
          WHERE o.library_id = ?1
            AND json_extract(change.value, '$.id') IN (SELECT id FROM related)
          ORDER BY o.hlc",
    )?;
    let rows = statement.query_map([library, edition_id], |row| {
        let hlc: String = row.get(2)?;
        Ok(HistoryEntry {
            operation_id: row.get(0)?,
            intent: row.get(1)?,
            timestamp: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: String,
    pub name: String,
    /// Distinct works this tag is linked to.
    pub works: i64,
}

pub fn list_tags(conn: &Connection, library: &str) -> Result<Vec<TagSummary>> {
    let mut statement = conn.prepare(
        "SELECT t.id, t.name, (SELECT count(DISTINCT wt.work_id) FROM work_tags wt
                                 WHERE wt.library_id = t.library_id AND wt.tag_id = t.id)
           FROM tags t WHERE t.library_id = ?1 ORDER BY t.name COLLATE NOCASE, t.id",
    )?;
    let rows = statement.query_map([library], |row| {
        Ok(TagSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            works: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn rename_tag(conn: &mut Connection, library: &str, id: &str, name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation(vec![FieldError::new(
            "name",
            ErrorCode::Required,
        )]));
    }
    if trimmed.chars().count() > MAX_TAG_LENGTH {
        return Err(Error::Validation(vec![FieldError::new(
            "name",
            ErrorCode::OutOfRange,
        )]));
    }

    let tx = conn.transaction()?;
    ensure_exists(&tx, library, Entity::Tag, id)?;
    let existing_id: Option<String> = tx
        .query_row(
            "SELECT id FROM tags WHERE library_id = ?1 AND name = ?2 COLLATE NOCASE
               AND id <> ?3",
            [library, trimmed, id],
            |row| row.get(0),
        )
        .optional()?;

    let mut plan = Plan::new(&tx, library);
    match existing_id {
        // Renaming to a name another tag already has merges them (Design: "Ya existe
        // ...: se unirán las dos"): move this tag's work links onto the existing one,
        // skipping works that already have it, then drop the now-empty tag.
        Some(target_id) => {
            let rows: Vec<(String, String)> = {
                let mut statement = tx.prepare(
                    "SELECT id, work_id FROM work_tags WHERE library_id = ?1 AND tag_id = ?2",
                )?;
                statement
                    .query_map([library, id], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<rusqlite::Result<_>>()?
            };
            for (row_id, work_id) in rows {
                let already_linked: bool = tx.query_row(
                    "SELECT EXISTS (SELECT 1 FROM work_tags
                       WHERE library_id = ?1 AND work_id = ?2 AND tag_id = ?3)",
                    rusqlite::params![library, work_id, target_id],
                    |row| row.get(0),
                )?;
                if already_linked {
                    plan.changes.push(Change::delete(Entity::WorkTag, row_id));
                } else {
                    plan.changes.push(Change::update(
                        Entity::WorkTag,
                        row_id,
                        json!({ "tag_id": target_id }),
                    ));
                }
            }
            plan.changes.push(Change::delete(Entity::Tag, id));
        }
        None => {
            plan.update_if_changed(Entity::Tag, id, json!({ "name": trimmed }))?;
        }
    }
    plan.record("tag.rename")?;
    tx.commit()?;
    Ok(())
}

pub fn delete_tag(conn: &mut Connection, library: &str, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, library, Entity::Tag, id)?;
    let mut changes: Vec<Change> = ids(
        &tx,
        "SELECT id FROM work_tags WHERE library_id = ?1 AND tag_id = ?2",
        library,
        id,
    )?
    .into_iter()
    .map(|row_id| Change::delete(Entity::WorkTag, row_id))
    .collect();
    changes.push(Change::delete(Entity::Tag, id));
    ops::record(&tx, library, "tag.remove", changes)?;
    tx.commit()?;
    Ok(())
}

fn split_list(joined: Option<String>) -> Vec<String> {
    joined
        .map(|s| s.split(LIST_SEPARATOR).map(str::to_owned).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, NewBook};

    use super::*;
    use crate::db;

    fn edition(title: &str, isbn: Option<&str>, authors: &[&str]) -> ValidEdition {
        EditionInput {
            title: title.to_owned(),
            authors: authors.iter().map(|a| a.to_string()).collect(),
            published_year: Some(2007),
            language: Some("es".to_owned()),
            isbn: isbn.map(str::to_owned),
            ..EditionInput::default()
        }
        .validate()
        .unwrap()
    }

    fn copy(owner: &str, format: Format) -> ValidCopy {
        CopyInput {
            owner: Some(owner.to_owned()),
            format,
            quantity: 1,
            location: None,
            acquired_on: None,
            platform: None,
            seasons: None,
        }
        .validate()
        .unwrap()
    }

    fn new_book(title: &str, isbn: Option<&str>, owner: &str, format: Format) -> ValidBook {
        ValidBook {
            edition: edition(title, isbn, &["Patrick Rothfuss"]),
            copy: copy(owner, format),
        }
    }

    fn intents(conn: &Connection) -> Vec<String> {
        let lib = ops::active_library(conn).unwrap();
        ids(
            conn,
            "SELECT intent FROM operations
              WHERE library_id = ?1 AND ?2 = ?2
                AND intent NOT IN ('device.register', 'library.create') ORDER BY hlc",
            &lib,
            "",
        )
        .unwrap()
    }

    fn last_changes(conn: &Connection) -> Vec<Change> {
        let json: String = conn
            .query_row(
                "SELECT changes FROM operations ORDER BY hlc DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn creates_work_edition_and_copy() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = new_book(
            "El nombre del viento",
            Some("0306406152"),
            "Tersoal",
            Format::Hardcover,
        );

        let created = create_book(&mut conn, &lib, &book, None).unwrap();
        let books = list_books(&conn, &lib, &BookFilter::default()).unwrap();

        assert!(!created.existing_edition);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "El nombre del viento");
        assert_eq!(books[0].authors, vec!["Patrick Rothfuss"]);
        assert_eq!(books[0].isbn13.as_deref(), Some("9780306406157"));
        assert_eq!(books[0].formats, vec![Format::Hardcover]);
        assert_eq!(books[0].owners, vec!["Tersoal"]);
    }

    #[test]
    fn check_isbn_finds_books_in_any_format() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Owned", Some("9780306406157"), "Ana", Format::Ebook),
            None,
        )
        .unwrap();

        let owned = check_isbn(&conn, "0-306-40615-2").unwrap();
        assert_eq!(owned.books.len(), 1);
        assert_eq!(owned.books[0].owners, vec!["Ana"]);

        let missing = check_isbn(&conn, "9780140328721").unwrap();
        assert!(missing.books.is_empty());

        assert!(matches!(check_isbn(&conn, "12"), Err(Error::Validation(_))));
    }

    #[test]
    fn same_isbn_adds_a_copy_to_the_existing_edition() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        let second = create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("0-306-40615-2"), "tersoal", Format::Ebook),
            None,
        )
        .unwrap();
        let books = list_books(&conn, &lib, &BookFilter::default()).unwrap();

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
    fn creates_a_missing_work_with_no_copy() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let missing = edition("Elantris", Some("9780765350374"), &["Brandon Sanderson"]);

        let created = create_missing(&mut conn, &lib, &missing, None).unwrap();
        let books = list_books(&conn, &lib, &BookFilter::default()).unwrap();

        assert!(!created.existing_edition);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Elantris");
        assert_eq!(books[0].copy_count, 0);
        assert_eq!(intents(&conn), vec!["work.add_missing"]);
    }

    #[test]
    fn creating_the_same_missing_work_twice_returns_the_existing_edition() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let missing = edition("Elantris", Some("9780765350374"), &["Brandon Sanderson"]);
        let first = create_missing(&mut conn, &lib, &missing, None).unwrap();

        let second = create_missing(&mut conn, &lib, &missing, None).unwrap();

        assert!(second.existing_edition);
        assert_eq!(second.edition_id, first.edition_id);
        assert_eq!(
            intents(&conn),
            vec!["work.add_missing"],
            "nothing recorded twice"
        );
    }

    #[test]
    fn holding_filters_owned_and_missing_works() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Owned", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        create_missing(
            &mut conn,
            &lib,
            &edition("Missing", Some("9780765350374"), &["Someone"]),
            None,
        )
        .unwrap();

        let owned = list_books(
            &conn,
            &lib,
            &BookFilter {
                holding: Some(Holding::Owned),
                ..Default::default()
            },
        )
        .unwrap();
        let missing = list_books(
            &conn,
            &lib,
            &BookFilter {
                holding: Some(Holding::Missing),
                ..Default::default()
            },
        )
        .unwrap();
        let everything = list_books(&conn, &lib, &BookFilter::default()).unwrap();

        assert_eq!(
            owned.iter().map(|b| &b.title).collect::<Vec<_>>(),
            vec!["Owned"]
        );
        assert_eq!(
            missing.iter().map(|b| &b.title).collect::<Vec<_>>(),
            vec!["Missing"]
        );
        assert_eq!(everything.len(), 2);
    }

    #[test]
    fn adding_and_removing_the_last_copy_moves_between_holdings() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_missing(
            &mut conn,
            &lib,
            &edition("Elantris", Some("9780765350374"), &["Brandon Sanderson"]),
            None,
        )
        .unwrap();

        let copy_id = add_copy(
            &mut conn,
            &lib,
            &created.edition_id,
            &copy("Tersoal", Format::Paperback),
        )
        .unwrap();
        let owned = list_books(
            &conn,
            &lib,
            &BookFilter {
                holding: Some(Holding::Owned),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(owned.len(), 1, "adding a copy makes it owned");

        delete_copy(&mut conn, &lib, &copy_id).unwrap();
        let missing = list_books(
            &conn,
            &lib,
            &BookFilter {
                holding: Some(Holding::Missing),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            missing.len(),
            1,
            "deleting its last copy makes it missing again"
        );
    }

    #[test]
    fn a_reader_can_only_request_a_missing_work() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let device = ops::device_id(&conn).unwrap();
        let (admin, reader) = (ops::new_id(), ops::new_id());
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.roles",
            vec![
                Change::create(
                    Entity::Member,
                    &admin,
                    json!({ "name": "Tersoal", "role": "admin" }),
                ),
                Change::create(
                    Entity::Member,
                    &reader,
                    json!({ "name": "Hugo", "role": "reader" }),
                ),
                Change::update(Entity::Device, &device, json!({ "member_id": reader })),
            ],
        )
        .unwrap();
        tx.commit().unwrap();

        let missing = edition("Elantris", Some("9780765350374"), &["Brandon Sanderson"]);
        let refused = create_missing(&mut conn, &lib, &missing, None);
        assert!(matches!(refused, Err(Error::NotAllowed)), "{refused:?}");
        assert!(
            list_books(&conn, &lib, &BookFilter::default())
                .unwrap()
                .is_empty()
        );

        let (_, requested) =
            ops::as_request(None, || create_missing(&mut conn, &lib, &missing, None)).unwrap();
        assert!(requested);
        assert!(
            list_books(&conn, &lib, &BookFilter::default())
                .unwrap()
                .is_empty(),
            "kept as a request, no work created"
        );
    }

    #[test]
    fn a_reader_asks_to_add_a_copy_of_a_missing_work_an_editor_adds_it_directly() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let missing = edition("Elantris", Some("9780765350374"), &["Brandon Sanderson"]);
        let edition_id = create_missing(&mut conn, &lib, &missing, None)
            .unwrap()
            .edition_id;

        let device = ops::device_id(&conn).unwrap();
        let (admin, reader) = (ops::new_id(), ops::new_id());
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.roles",
            vec![
                Change::create(
                    Entity::Member,
                    &admin,
                    json!({ "name": "Tersoal", "role": "admin" }),
                ),
                Change::create(
                    Entity::Member,
                    &reader,
                    json!({ "name": "Hugo", "role": "reader" }),
                ),
                Change::update(Entity::Device, &device, json!({ "member_id": reader })),
            ],
        )
        .unwrap();
        tx.commit().unwrap();

        let new_copy = copy("Hugo", Format::Paperback);
        let refused = add_copy(&mut conn, &lib, &edition_id, &new_copy);
        assert!(matches!(refused, Err(Error::NotAllowed)), "{refused:?}");

        let (_, requested) =
            ops::as_request(None, || add_copy(&mut conn, &lib, &edition_id, &new_copy)).unwrap();
        assert!(requested);
        assert!(
            get_book(&conn, &lib, &edition_id)
                .unwrap()
                .copies
                .is_empty(),
            "kept as a request, no copy added"
        );

        // Promoting Hugo is an administrator's call, not something this device (his)
        // can do to itself; the row is set directly, since only his new role matters
        // here, not how he came to have it.
        conn.execute(
            "UPDATE members SET role = 'editor' WHERE library_id = ?1 AND id = ?2",
            rusqlite::params![lib, &reader],
        )
        .unwrap();
        add_copy(&mut conn, &lib, &edition_id, &new_copy).unwrap();
        assert_eq!(get_book(&conn, &lib, &edition_id).unwrap().copies.len(), 1);
    }

    #[test]
    fn books_without_isbn_are_never_merged() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("B", None, "Tersoal", Format::Paperback),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("B", None, "Tersoal", Format::Paperback),
            None,
        )
        .unwrap();

        assert_eq!(
            list_books(&conn, &lib, &BookFilter::default())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn every_write_is_logged_and_history_follows_the_edition() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Ebook),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Other", None, "Tersoal", Format::Ebook),
            None,
        )
        .unwrap();

        let history = book_history(&conn, &lib, &created.edition_id).unwrap();

        let intents: Vec<&str> = history.iter().map(|entry| entry.intent.as_str()).collect();
        assert_eq!(intents, vec!["book.add", "copy.add"]);
        assert!(history[0].timestamp <= history[1].timestamp);
    }

    #[test]
    fn get_book_returns_edition_and_copies() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        add_copy(
            &mut conn,
            &lib,
            &created.edition_id,
            &copy("Ana", Format::Ebook),
        )
        .unwrap();

        let book = get_book(&conn, &lib, &created.edition_id).unwrap();

        assert_eq!(book.title, "A");
        assert_eq!(book.language.as_deref(), Some("es"));
        assert_eq!(book.copies.len(), 2);
        assert_eq!(book.copies[1].owner.as_deref(), Some("Ana"));
        assert_eq!(book.copies[1].format, Format::Ebook);
    }

    /// The platform belongs to each copy, not the edition (D08): one game can have
    /// copies on different platforms.
    #[test]
    fn a_video_game_with_two_copies_on_different_platforms_round_trips() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let game = EditionInput {
            title: "Hollow Knight".to_owned(),
            kind: Kind::Videogame,
            ..EditionInput::default()
        }
        .validate()
        .unwrap();
        let switch_copy = CopyInput {
            owner: None,
            format: Format::Physical,
            quantity: 1,
            location: None,
            acquired_on: None,
            platform: Some("Nintendo Switch".to_owned()),
            seasons: None,
        }
        .validate()
        .unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &ValidBook {
                edition: game,
                copy: switch_copy,
            },
            None,
        )
        .unwrap();
        let wiiu_copy = CopyInput {
            owner: None,
            format: Format::Physical,
            quantity: 1,
            location: None,
            acquired_on: None,
            platform: Some("Wii U".to_owned()),
            seasons: None,
        }
        .validate()
        .unwrap();
        add_copy(&mut conn, &lib, &created.edition_id, &wiiu_copy).unwrap();

        let book = get_book(&conn, &lib, &created.edition_id).unwrap();

        assert_eq!(book.copies.len(), 2);
        assert_eq!(book.copies[0].platform.as_deref(), Some("Nintendo Switch"));
        assert_eq!(book.copies[1].platform.as_deref(), Some("Wii U"));
    }

    /// A TV series' total seasons stay on the edition; which ones a copy holds is
    /// free text on the copy.
    #[test]
    fn a_series_copy_with_the_seasons_it_holds_round_trips() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series = EditionInput {
            title: "The Wire".to_owned(),
            kind: Kind::TvSeries,
            seasons: Some(5),
            ..EditionInput::default()
        }
        .validate()
        .unwrap();
        let dvd_copy = CopyInput {
            owner: None,
            format: Format::Dvd,
            quantity: 1,
            location: None,
            acquired_on: None,
            platform: None,
            seasons: Some("1–2".to_owned()),
        }
        .validate()
        .unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &ValidBook {
                edition: series,
                copy: dvd_copy,
            },
            None,
        )
        .unwrap();

        let book = get_book(&conn, &lib, &created.edition_id).unwrap();

        assert_eq!(book.seasons, Some(5));
        assert_eq!(book.copies[0].seasons.as_deref(), Some("1–2"));
    }

    #[test]
    fn editing_records_only_changed_fields() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let mut edited = edition("A", None, &["Patrick Rothfuss"]);
        edited.published_year = Some(2011);

        update_edition(&mut conn, &lib, &created.edition_id, &edited, None).unwrap();

        let changes = last_changes(&conn);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].entity, Entity::Edition);
        assert_eq!(
            Value::Object(changes[0].fields.clone()),
            json!({ "published_year": 2011 })
        );
        assert_eq!(
            get_book(&conn, &lib, &created.edition_id)
                .unwrap()
                .published_year,
            Some(2011)
        );
    }

    #[test]
    fn notes_round_trip_and_can_be_cleared() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let mut book = new_book("A", None, "Tersoal", Format::Hardcover);
        book.edition.notes = Some("Falta una vela.".into());
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        assert_eq!(
            get_book(&conn, &lib, &created.edition_id).unwrap().notes,
            Some("Falta una vela.".into())
        );

        let mut cleared = edition("A", None, &["Patrick Rothfuss"]);
        cleared.notes = None;
        update_edition(&mut conn, &lib, &created.edition_id, &cleared, None).unwrap();

        assert_eq!(
            get_book(&conn, &lib, &created.edition_id).unwrap().notes,
            None
        );
    }

    #[test]
    fn editing_without_changes_records_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        update_edition(
            &mut conn,
            &lib,
            &created.edition_id,
            &edition("A", None, &["Patrick Rothfuss"]),
            None,
        )
        .unwrap();

        assert_eq!(intents(&conn), vec!["book.add"]);
    }

    #[test]
    fn editing_the_title_of_a_single_edition_renames_its_work() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("Old", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        update_edition(
            &mut conn,
            &lib,
            &created.edition_id,
            &edition("New", None, &["Patrick Rothfuss"]),
            None,
        )
        .unwrap();

        let work_title: String = conn
            .query_row("SELECT title FROM works", [], |r| r.get(0))
            .unwrap();
        assert_eq!(work_title, "New");
    }

    #[test]
    fn editing_authors_adds_reorders_and_removes() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let mut book = new_book("A", None, "Tersoal", Format::Hardcover);
        book.edition.authors = vec!["Ana".into(), "Bea".into(), "Carla".into()];
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        update_edition(
            &mut conn,
            &lib,
            &created.edition_id,
            &edition("A", None, &["Carla", "Ana", "Dora"]),
            None,
        )
        .unwrap();

        assert_eq!(
            get_book(&conn, &lib, &created.edition_id).unwrap().authors,
            vec!["Carla", "Ana", "Dora"]
        );
        let people: i64 = conn
            .query_row("SELECT count(*) FROM people", [], |r| r.get(0))
            .unwrap();
        assert_eq!(people, 4, "removed authors stay in the people catalogue");
    }

    #[test]
    fn isbn_of_another_edition_is_rejected() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let second = create_book(
            &mut conn,
            &lib,
            &new_book("B", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        let error = update_edition(
            &mut conn,
            &lib,
            &second.edition_id,
            &edition("B", Some("9780306406157"), &[]),
            None,
        )
        .unwrap_err();

        assert!(
            matches!(error, Error::Validation(errors) if errors[0].code == ErrorCode::IsbnInUse)
        );
    }

    #[test]
    fn copies_can_be_edited_and_removed() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let copy_id = get_book(&conn, &lib, &created.edition_id).unwrap().copies[0]
            .id
            .clone();

        let mut moved = copy("Tersoal", Format::Hardcover);
        moved.location = Some("Salón, balda 2".into());
        update_copy(&mut conn, &lib, &copy_id, &moved).unwrap();
        assert_eq!(
            Value::Object(last_changes(&conn)[0].fields.clone()),
            json!({ "location": "Salón, balda 2" })
        );

        delete_copy(&mut conn, &lib, &copy_id).unwrap();
        assert!(
            get_book(&conn, &lib, &created.edition_id)
                .unwrap()
                .copies
                .is_empty()
        );
        assert_eq!(intents(&conn), vec!["book.add", "copy.edit", "copy.remove"]);
    }

    #[test]
    fn deleting_a_book_removes_everything_but_people_and_members() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        add_copy(
            &mut conn,
            &lib,
            &created.edition_id,
            &copy("Ana", Format::Ebook),
        )
        .unwrap();

        delete_book(&mut conn, &lib, &created.edition_id).unwrap();

        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        for table in [
            "works",
            "work_contributors",
            "editions",
            "book_details",
            "copies",
        ] {
            assert_eq!(count(table), 0, "{table} should be empty");
        }
        assert_eq!(count("people"), 1);
        assert_eq!(count("members"), 2);
        // The same ISBN can be catalogued again.
        create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
    }

    #[test]
    fn missing_rows_are_reported() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let unknown = new_id();
        assert!(matches!(
            delete_book(&mut conn, &lib, &unknown),
            Err(Error::MissingRow { .. })
        ));
        assert!(matches!(
            delete_copy(&mut conn, &lib, &unknown),
            Err(Error::MissingRow { .. })
        ));
        assert!(matches!(
            get_book(&conn, &lib, &unknown),
            Err(Error::MissingRow { .. })
        ));
    }

    #[test]
    fn edition_matching_candidate_checks_isbn_then_barcode() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let with_barcode = ValidBook {
            edition: EditionInput {
                title: "B".to_owned(),
                barcode: Some("4006381333931".to_owned()),
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: copy("Tersoal", Format::Box),
        };
        create_book(&mut conn, &lib, &with_barcode, None).unwrap();

        // Same ISBN in another format ("0-306-...") still matches.
        assert!(
            edition_matching_candidate(&conn, &lib, Some("0-306-40615-2"), None)
                .unwrap()
                .is_some()
        );
        assert!(
            edition_matching_candidate(&conn, &lib, None, Some("4006381333931"))
                .unwrap()
                .is_some()
        );
        assert_eq!(
            edition_matching_candidate(&conn, &lib, Some("9780000000002"), None).unwrap(),
            None
        );
        assert_eq!(
            edition_matching_candidate(&conn, &lib, None, None).unwrap(),
            None
        );
    }

    #[test]
    fn new_book_deserializes_from_ui_json() {
        let book: NewBook = serde_json::from_value(json!({
            "title": "Dune", "copy": { "format": "pocket" }
        }))
        .unwrap();
        assert_eq!(book.validate().unwrap().edition.title, "Dune");
    }

    fn search(conn: &Connection, query: &str) -> Vec<String> {
        let lib = ops::active_library(conn).unwrap();
        let filter = BookFilter {
            query: Some(query.into()),
            ..BookFilter::default()
        };
        list_books(conn, &lib, &filter)
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect()
    }

    #[test]
    fn search_ignores_accents_and_case_and_matches_prefixes() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let mut book = new_book(
            "El señor de los anillos",
            None,
            "Tersoal",
            Format::Hardcover,
        );
        book.edition.authors = vec!["J. R. R. Tolkien".into()];
        book.edition.publisher = Some("Minotauro".into());
        create_book(&mut conn, &lib, &book, None).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Dune", None, "Tersoal", Format::Paperback),
            None,
        )
        .unwrap();

        assert_eq!(search(&conn, "SENOR"), vec!["El señor de los anillos"]);
        assert_eq!(search(&conn, "tolk anill"), vec!["El señor de los anillos"]);
        assert_eq!(search(&conn, "minotauro"), vec!["El señor de los anillos"]);
        assert!(
            search(&conn, "tolkien dune").is_empty(),
            "every word must match"
        );
        assert_eq!(
            search(&conn, "   ").len(),
            2,
            "an empty query lists everything"
        );
    }

    #[test]
    fn search_finds_isbn_in_either_format() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        assert_eq!(search(&conn, "0-306-40615-2"), vec!["A"]);
        assert_eq!(search(&conn, "978-0-306-40615-7"), vec!["A"]);
    }

    #[test]
    fn index_follows_edits_renames_and_deletions() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let created = create_book(
            &mut conn,
            &lib,
            &new_book("Old title", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        update_edition(
            &mut conn,
            &lib,
            &created.edition_id,
            &edition("New title", None, &["Ursula"]),
            None,
        )
        .unwrap();
        assert!(search(&conn, "old").is_empty());
        assert_eq!(search(&conn, "ursula"), vec!["New title"]);

        // Renaming a person reindexes their books.
        let person_id: String = conn
            .query_row("SELECT id FROM people WHERE name = 'Ursula'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "person.rename",
            vec![Change::update(
                Entity::Person,
                &person_id,
                json!({ "name": "Le Guin" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        assert_eq!(search(&conn, "guin"), vec!["New title"]);

        delete_book(&mut conn, &lib, &created.edition_id).unwrap();
        assert!(search(&conn, "new").is_empty());
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM search_index", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn copy_filters_need_one_copy_matching_all_of_them() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let mut a = new_book("A", None, "Ana", Format::Hardcover);
        a.copy.location = Some("Salón".into());
        create_book(&mut conn, &lib, &a, None).unwrap();
        let b = create_book(
            &mut conn,
            &lib,
            &new_book("B", None, "Ana", Format::Ebook),
            None,
        )
        .unwrap();
        add_copy(
            &mut conn,
            &lib,
            &b.edition_id,
            &copy("Bea", Format::Hardcover),
        )
        .unwrap();
        let ana = "ana".to_owned();

        let titles = |filter: BookFilter| -> Vec<String> {
            list_books(&conn, &lib, &filter)
                .unwrap()
                .into_iter()
                .map(|b| b.title)
                .collect()
        };
        assert_eq!(
            titles(BookFilter {
                format: Some(Format::Hardcover),
                ..Default::default()
            }),
            vec!["A", "B"]
        );
        assert_eq!(
            titles(BookFilter {
                owner: Some(ana.clone()),
                ..Default::default()
            }),
            vec!["A", "B"]
        );
        assert_eq!(
            titles(BookFilter {
                format: Some(Format::Hardcover),
                owner: Some(ana),
                ..Default::default()
            }),
            vec!["A"],
            "B has a hardcover (Bea's) and a copy of Ana's (ebook), but no hardcover of Ana's"
        );
        assert_eq!(
            titles(BookFilter {
                location: Some("Salón".into()),
                ..Default::default()
            }),
            vec!["A"]
        );
    }

    #[test]
    fn wished_by_lists_who_wishes_a_work_with_me_first() {
        use crate::marks::{self, Mark};
        use crate::people::PersonPick;

        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Dune", None, "Ana", Format::Hardcover),
            None,
        )
        .unwrap();
        let edition_id = list_books(&conn, &lib, &BookFilter::default())
            .unwrap()
            .into_iter()
            .find(|b| b.title == "Dune")
            .unwrap()
            .edition_id;
        let work_id = get_book(&conn, &lib, &edition_id).unwrap().work_id;

        marks::claim_person(
            &mut conn,
            &lib,
            &PersonPick {
                id: None,
                name: "Tersoal".into(),
            },
        )
        .unwrap();
        marks::set_mark(
            &mut conn,
            &lib,
            &work_id,
            &Mark {
                wished: true,
                ..Mark::default()
            },
        )
        .unwrap();

        let bea = new_id();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                &bea,
                json!({ "name": "Bea" }),
            )],
        )
        .unwrap();
        ops::record(
            &tx,
            &lib,
            "mark.set",
            vec![Change::create(
                Entity::Mark,
                kosmotheka_domain::mark_id(&bea, &work_id),
                json!({ "member_id": bea, "work_id": work_id, "wished": true }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();

        let summary = list_books(&conn, &lib, &BookFilter::default())
            .unwrap()
            .into_iter()
            .find(|b| b.title == "Dune")
            .unwrap();
        assert_eq!(summary.wished_by, vec!["me", "Bea"]);
        let detail = get_book(&conn, &lib, &edition_id).unwrap();
        assert_eq!(detail.wished_by, vec!["me", "Bea"]);
    }

    #[test]
    fn wished_marks_are_filtered_by_who_wishes() {
        use crate::marks::{self, Mark};
        use crate::people::PersonPick;

        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Dune", None, "Ana", Format::Hardcover),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("Wingspan", None, "Ana", Format::Hardcover),
            None,
        )
        .unwrap();
        fn work(conn: &Connection, title: &str) -> String {
            let lib = ops::active_library(conn).unwrap();
            let edition = list_books(conn, &lib, &BookFilter::default())
                .unwrap()
                .into_iter()
                .find(|b| b.title == title)
                .unwrap()
                .edition_id;
            get_book(conn, &lib, &edition).unwrap().work_id
        }
        let dune = work(&conn, "Dune");

        marks::claim_person(
            &mut conn,
            &lib,
            &PersonPick {
                id: None,
                name: "Tersoal".into(),
            },
        )
        .unwrap();
        marks::set_mark(
            &mut conn,
            &lib,
            &dune,
            &Mark {
                wished: true,
                ..Mark::default()
            },
        )
        .unwrap();

        fn titles(conn: &Connection, filter: BookFilter) -> Vec<String> {
            let lib = ops::active_library(conn).unwrap();
            list_books(conn, &lib, &filter)
                .unwrap()
                .into_iter()
                .map(|b| b.title)
                .collect()
        }
        assert_eq!(
            titles(
                &conn,
                BookFilter {
                    my_wished: true,
                    ..Default::default()
                }
            ),
            vec!["Dune"]
        );
        assert!(
            titles(
                &conn,
                BookFilter {
                    others_wished: true,
                    ..Default::default()
                }
            )
            .is_empty(),
            "the only wish is mine"
        );
        assert!(
            list_books(&conn, &lib, &BookFilter::default())
                .unwrap()
                .into_iter()
                .find(|b| b.title == "Dune")
                .unwrap()
                .my_wished
        );

        // Someone else wishes Wingspan: seen as "others", not "mine".
        let wingspan = work(&conn, "Wingspan");
        let tx = conn.transaction().unwrap();
        let bea = new_id();
        ops::record(
            &tx,
            &lib,
            "person.claim",
            vec![kosmotheka_domain::Change::create(
                kosmotheka_domain::Entity::Member,
                &bea,
                json!({ "name": "Bea" }),
            )],
        )
        .unwrap();
        ops::record(
            &tx,
            &lib,
            "mark.set",
            vec![kosmotheka_domain::Change::create(
                kosmotheka_domain::Entity::Mark,
                kosmotheka_domain::mark_id(&bea, &wingspan),
                json!({ "member_id": bea, "work_id": wingspan, "wished": true }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();

        assert_eq!(
            titles(
                &conn,
                BookFilter {
                    others_wished: true,
                    ..Default::default()
                }
            ),
            vec!["Wingspan"]
        );
        assert_eq!(
            titles(
                &conn,
                BookFilter {
                    my_wished: true,
                    ..Default::default()
                }
            ),
            vec!["Dune"],
            "Bea's wish is not mine"
        );
    }

    #[test]
    fn publishers_are_a_shared_catalogue() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        for (title, publisher) in [("A", "Minotauro"), ("B", "minotauro")] {
            let mut book = new_book(title, None, "Tersoal", Format::Paperback);
            book.edition.publisher = Some(publisher.into());
            create_book(&mut conn, &lib, &book, None).unwrap();
        }

        let options = catalog_options(&conn, &lib).unwrap();

        assert_eq!(options.publishers, vec!["Minotauro"]);
        let books = list_books(&conn, &lib, &BookFilter::default()).unwrap();
        assert!(
            books
                .iter()
                .all(|b| b.publisher.as_deref() == Some("Minotauro"))
        );
    }

    fn boardgame(title: &str, barcode: Option<&str>, designer: &str) -> ValidEdition {
        EditionInput {
            title: title.to_owned(),
            kind: Kind::Boardgame,
            contributors: vec![ContributorInput {
                name: designer.to_owned(),
                role: ContributorRole::Designer,
            }],
            barcode: barcode.map(str::to_owned),
            players_min: Some(2),
            players_max: Some(5),
            duration_minutes: Some(60),
            min_age: Some(10),
            ..EditionInput::default()
        }
        .validate()
        .unwrap()
    }

    #[test]
    fn a_boardgame_with_designer_barcode_and_details_round_trips() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: boardgame("Wingspan", Some("4006381333931"), "Elizabeth Hargrave"),
            copy: copy("Tersoal", Format::Box),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let detail = get_book(&conn, &lib, &created.edition_id).unwrap();

        assert_eq!(detail.kind, Kind::Boardgame);
        assert_eq!(detail.barcode.as_deref(), Some("4006381333931"));
        assert_eq!(
            detail.contributors,
            vec![ContributorDetail {
                name: "Elizabeth Hargrave".into(),
                role: ContributorRole::Designer,
            }]
        );
        assert_eq!(detail.players_min, Some(2));
        assert_eq!(detail.players_max, Some(5));
        assert_eq!(detail.duration_minutes, Some(60));
        assert_eq!(detail.min_age, Some(10));
    }

    #[test]
    fn the_same_barcode_adds_a_copy_to_the_existing_edition() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: boardgame("Wingspan", Some("4006381333931"), "Elizabeth Hargrave"),
            copy: copy("Tersoal", Format::Box),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let second = ValidBook {
            edition: boardgame("Wingspan", Some("4006381333931"), "Elizabeth Hargrave"),
            copy: copy("Laura", Format::Box),
        };
        let result = create_book(&mut conn, &lib, &second, None).unwrap();

        assert!(result.existing_edition);
        assert_eq!(result.edition_id, created.edition_id);
        assert_eq!(
            get_book(&conn, &lib, &created.edition_id)
                .unwrap()
                .copies
                .len(),
            2
        );
    }

    #[test]
    fn editing_players_updates_only_that_field() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: boardgame("Wingspan", None, "Elizabeth Hargrave"),
            copy: copy("Tersoal", Format::Box),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let mut edited = boardgame("Wingspan", None, "Elizabeth Hargrave");
        edited.details = kosmotheka_domain::KindDetails::Boardgame {
            players_min: Some(2),
            players_max: Some(7),
            duration_minutes: Some(60),
            min_age: Some(10),
        };

        update_edition(&mut conn, &lib, &created.edition_id, &edited, None).unwrap();

        let changes = last_changes(&conn);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].entity, Entity::BoardgameDetails);
        assert_eq!(
            Value::Object(changes[0].fields.clone()),
            json!({ "players_max": 7 })
        );
        assert_eq!(
            get_book(&conn, &lib, &created.edition_id)
                .unwrap()
                .players_max,
            Some(7)
        );
    }

    #[test]
    fn changing_the_kind_moves_the_details_to_the_new_kinds_table() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: boardgame("Dune", None, "Someone"),
            copy: copy("Tersoal", Format::Box),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let film = EditionInput {
            title: "Dune".to_owned(),
            kind: Kind::Film,
            duration_minutes: Some(155),
            ..EditionInput::default()
        }
        .validate()
        .unwrap();
        update_edition(&mut conn, &lib, &created.edition_id, &film, None).unwrap();

        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(count("boardgame_details"), 0);
        assert_eq!(count("film_details"), 1);
        let detail = get_book(&conn, &lib, &created.edition_id).unwrap();
        assert_eq!(detail.kind, Kind::Film);
        assert_eq!(detail.duration_minutes, Some(155));
    }

    #[test]
    fn book_filter_by_kind_returns_only_that_kind() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &ValidBook {
                edition: boardgame("Wingspan", None, "Elizabeth Hargrave"),
                copy: copy("Tersoal", Format::Box),
            },
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &lib,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        let titles: Vec<String> = list_books(
            &conn,
            &lib,
            &BookFilter {
                kind: Some(Kind::Boardgame),
                ..Default::default()
            },
        )
        .unwrap()
        .into_iter()
        .map(|b| b.title)
        .collect();
        assert_eq!(titles, vec!["Wingspan"]);
    }

    #[test]
    fn moving_a_boardgame_keeps_kind_designer_barcode_and_details() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: boardgame("Wingspan", Some("4006381333931"), "Elizabeth Hargrave"),
            copy: copy("Tersoal", Format::Box),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let other_library = crate::libraries::create_library(&mut conn, "Other").unwrap();

        let new_edition = move_book(&mut conn, &lib, &created.edition_id, &other_library).unwrap();

        let detail = get_book(&conn, &other_library, &new_edition).unwrap();
        assert_eq!(detail.kind, Kind::Boardgame);
        assert_eq!(detail.barcode.as_deref(), Some("4006381333931"));
        assert_eq!(
            detail.contributors,
            vec![ContributorDetail {
                name: "Elizabeth Hargrave".into(),
                role: ContributorRole::Designer,
            }]
        );
        assert_eq!(detail.players_min, Some(2));
        assert_eq!(detail.duration_minutes, Some(60));
    }

    fn with_genres_and_tags(title: &str, genres: &[&str], tags: &[&str]) -> ValidEdition {
        EditionInput {
            title: title.to_owned(),
            genres: genres.iter().map(|g| g.to_string()).collect(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..EditionInput::default()
        }
        .validate()
        .unwrap()
    }

    #[test]
    fn genres_and_tags_are_saved_and_read_back() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: with_genres_and_tags("A", &["fantasy", "adventure"], &["favorite", "gift"]),
            copy: copy("Tersoal", Format::Hardcover),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let detail = get_book(&conn, &lib, &created.edition_id).unwrap();
        assert_eq!(detail.genres, vec!["adventure", "fantasy"]);
        assert_eq!(detail.tags, vec!["favorite", "gift"]);

        let summary = list_books(&conn, &lib, &BookFilter::default()).unwrap();
        assert_eq!(summary[0].tags, vec!["favorite", "gift"]);
    }

    #[test]
    fn unknown_genre_on_create_is_rejected() {
        let input = EditionInput {
            title: "A".to_owned(),
            genres: vec!["not_a_genre".into()],
            ..EditionInput::default()
        };
        assert!(matches!(
            input.validate().unwrap_err()[0].code,
            ErrorCode::InvalidGenre
        ));
    }

    #[test]
    fn a_genre_of_another_kind_is_dropped_when_saved() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let edition = EditionInput {
            title: "Wingspan".to_owned(),
            kind: Kind::Boardgame,
            // "manga" belongs to comic, not boardgame.
            genres: vec!["manga".into(), "strategy".into()],
            ..EditionInput::default()
        }
        .validate()
        .unwrap();
        let book = ValidBook {
            edition,
            copy: copy("Tersoal", Format::Box),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        let detail = get_book(&conn, &lib, &created.edition_id).unwrap();
        assert_eq!(detail.genres, vec!["strategy"]);
    }

    #[test]
    fn updating_genres_and_tags_removes_and_adds() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: with_genres_and_tags("A", &["fantasy"], &["favorite"]),
            copy: copy("Tersoal", Format::Hardcover),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();

        update_edition(
            &mut conn,
            &lib,
            &created.edition_id,
            &with_genres_and_tags("A", &["horror"], &["gift"]),
            None,
        )
        .unwrap();

        let detail = get_book(&conn, &lib, &created.edition_id).unwrap();
        assert_eq!(detail.genres, vec!["horror"]);
        assert_eq!(detail.tags, vec!["gift"]);
    }

    #[test]
    fn the_same_tag_name_reuses_the_tag() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        for title in ["A", "B"] {
            let book = ValidBook {
                edition: with_genres_and_tags(title, &[], &["Favorite"]),
                copy: copy("Tersoal", Format::Hardcover),
            };
            create_book(&mut conn, &lib, &book, None).unwrap();
        }
        // A second spelling reuses the same row case-insensitively.
        let book = ValidBook {
            edition: with_genres_and_tags("C", &[], &["favorite"]),
            copy: copy("Tersoal", Format::Hardcover),
        };
        create_book(&mut conn, &lib, &book, None).unwrap();

        let tags = list_tags(&conn, &lib).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Favorite");
        assert_eq!(tags[0].works, 3);
    }

    #[test]
    fn filters_by_genre_and_by_tag() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        create_book(
            &mut conn,
            &lib,
            &ValidBook {
                edition: with_genres_and_tags("A", &["fantasy"], &["favorite"]),
                copy: copy("Tersoal", Format::Hardcover),
            },
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &lib,
            &ValidBook {
                edition: with_genres_and_tags("B", &["horror"], &["gift"]),
                copy: copy("Tersoal", Format::Hardcover),
            },
            None,
        )
        .unwrap();

        let titles = |filter: BookFilter| -> Vec<String> {
            list_books(&conn, &lib, &filter)
                .unwrap()
                .into_iter()
                .map(|b| b.title)
                .collect()
        };
        assert_eq!(
            titles(BookFilter {
                genres: vec!["fantasy".into()],
                ..Default::default()
            }),
            vec!["A"]
        );
        let gift_tag = list_tags(&conn, &lib)
            .unwrap()
            .into_iter()
            .find(|t| t.name == "gift")
            .unwrap()
            .id;
        assert_eq!(
            titles(BookFilter {
                tags: vec![gift_tag],
                ..Default::default()
            }),
            vec!["B"]
        );
    }

    #[test]
    fn renaming_and_deleting_a_tag() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let book = ValidBook {
            edition: with_genres_and_tags("A", &[], &["favorite"]),
            copy: copy("Tersoal", Format::Hardcover),
        };
        let created = create_book(&mut conn, &lib, &book, None).unwrap();
        let tag_id = list_tags(&conn, &lib).unwrap()[0].id.clone();

        rename_tag(&mut conn, &lib, &tag_id, "  Top pick  ").unwrap();
        assert_eq!(list_tags(&conn, &lib).unwrap()[0].name, "Top pick");

        delete_tag(&mut conn, &lib, &tag_id).unwrap();
        assert!(list_tags(&conn, &lib).unwrap().is_empty());
        assert!(
            get_book(&conn, &lib, &created.edition_id)
                .unwrap()
                .tags
                .is_empty()
        );
        // The work itself stays.
        assert_eq!(
            get_book(&conn, &lib, &created.edition_id).unwrap().title,
            "A"
        );
    }

    #[test]
    fn renaming_a_tag_to_an_existing_name_merges_them() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let a = ValidBook {
            edition: with_genres_and_tags("A", &[], &["favorite", "gift"]),
            copy: copy("Tersoal", Format::Hardcover),
        };
        let b = ValidBook {
            edition: with_genres_and_tags("B", &[], &["favorite"]),
            copy: copy("Tersoal", Format::Hardcover),
        };
        let created_a = create_book(&mut conn, &lib, &a, None).unwrap();
        create_book(&mut conn, &lib, &b, None).unwrap();
        let tags = list_tags(&conn, &lib).unwrap();
        let favorite = tags.iter().find(|t| t.name == "favorite").unwrap();
        let gift = tags.iter().find(|t| t.name == "gift").unwrap();

        // Case-insensitive match, same as a plain rename.
        rename_tag(&mut conn, &lib, &favorite.id, "GIFT").unwrap();

        let remaining = list_tags(&conn, &lib).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, gift.id);
        assert_eq!(remaining[0].name, "gift");
        // A had both tags: merging must not duplicate its link to "gift".
        assert_eq!(remaining[0].works, 2);
        let book_a = get_book(&conn, &lib, &created_a.edition_id).unwrap();
        assert_eq!(book_a.tags, vec!["gift"]);
    }
}
