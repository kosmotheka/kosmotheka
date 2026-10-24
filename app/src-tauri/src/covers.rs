//! Cover images stored as local files named by the sha256 of their bytes (D27).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kosmotheka_domain::{Change, Entity};
use reqwest::header::CONTENT_TYPE;
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;
use sha2::{Digest, Sha256};
use url::Url;

use crate::db::Database;
use crate::error::{Error, Result};
use crate::ops;

pub const MAX_COVER_BYTES: usize = 5 * 1024 * 1024;

/// The longest side of a stored cover, and the WebP quality it is saved with (D82).
pub const MAX_SIDE: u32 = 800;
const QUALITY: f32 = 75.0;
/// Far beyond any real cover, and small enough that a crafted file is refused before it can exhaust memory.
const MAX_INPUT_SIDE: u32 = 12_000;
const MAX_DECODE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone)]
pub struct Covers {
    dir: PathBuf,
}

impl Covers {
    pub fn new(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Downloads an already validated URL (see `kosmotheka_domain::parse_cover_url`)
    /// and returns the stored file name.
    pub async fn download(&self, client: &reqwest::Client, url: &Url) -> Result<String> {
        let response = client.get(url.clone()).send().await?.error_for_status()?;
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        // Only a first check: what is kept is the reduced WebP, whatever the type.
        extension_for(&content_type).ok_or(Error::CoverRejected("not an image"))?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_COVER_BYTES as u64)
        {
            return Err(Error::CoverRejected("too large"));
        }
        let bytes = response.bytes().await?;
        if bytes.len() > MAX_COVER_BYTES {
            return Err(Error::CoverRejected("too large"));
        }
        self.store_cover(&bytes)
    }

    /// Reduces `bytes` to the WebP that is kept (D82) and returns its file name.
    pub fn store_cover(&self, bytes: &[u8]) -> Result<String> {
        self.store(&normalize(bytes)?, "webp")
    }

    /// Whether a stored cover is already what `normalize` makes: a WebP within `MAX_SIDE`. Only its header is read.
    pub fn is_reduced(&self, name: &str) -> bool {
        is_cover_name(name)
            && name.rsplit_once('.').map(|(_, extension)| extension) == Some("webp")
            && image::ImageReader::open(self.dir.join(name))
                .ok()
                .and_then(|reader| reader.with_guessed_format().ok())
                .and_then(|reader| reader.into_dimensions().ok())
                .is_some_and(|(width, height)| width <= MAX_SIDE && height <= MAX_SIDE)
    }

    /// The reduced file for a cover stored before D82, or `None` when there is nothing to do here: it is reduced
    /// already, or its file is not on this device (it will arrive from one that has it).
    pub fn reduce_existing(&self, name: &str) -> Result<Option<String>> {
        if !is_cover_name(name) || self.is_reduced(name) {
            return Ok(None);
        }
        match std::fs::read(self.dir.join(name)) {
            Ok(bytes) => self.store_cover(&bytes).map(Some),
            Err(_) => Ok(None),
        }
    }

    /// Same bytes, same name: storing a cover twice is harmless.
    pub fn store(&self, bytes: &[u8], extension: &str) -> Result<String> {
        let name = format!("{}.{extension}", sha256_hex(bytes));
        let path = self.dir.join(&name);
        if !path.exists() {
            // Write to a temporary file first so a crash never leaves half an image.
            let temporary = self.dir.join(format!(".{name}.part"));
            std::fs::write(&temporary, bytes)?;
            std::fs::rename(&temporary, &path)?;
        }
        Ok(name)
    }
}

/// `<sha256>.<ext>`, as `store` names every cover: anything else is never used as a path.
pub fn is_cover_name(name: &str) -> bool {
    let Some((hash, extension)) = name.split_once('.') else {
        return false;
    };
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        && matches!(extension, "jpg" | "png" | "webp" | "gif")
}

/// Edition covers changed at a time, in one operation each.
const BATCH: usize = 100;

/// An edition whose cover was stored before D82, and the reduced file it moves to.
pub struct Reduction {
    library: String,
    edition: String,
    from: String,
    to: String,
}

/// Reduces the covers stored before D82 and points their editions to the reduced files (milestone D82, part 1). It
/// runs at start in the background: the database is locked only to read the list and to record, never while images
/// are encoded. Returns how many editions changed.
pub fn reduce_stored(database: &Database, covers: &Covers) -> Result<usize> {
    let stored = stored_covers(&*database.lock()?)?;
    let reductions = reduce_files(covers, stored);
    if reductions.is_empty() {
        return Ok(0);
    }
    record_reductions(&mut *database.lock()?, &reductions)
}

/// Every edition with a cover: library, edition and file name.
fn stored_covers(conn: &Connection) -> Result<Vec<(String, String, String)>> {
    Ok(conn
        .prepare("SELECT library_id, id, cover FROM editions WHERE cover IS NOT NULL")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?)
}

/// Each file is reduced once, however many editions share it. A file that cannot be read as an image is left alone.
fn reduce_files(covers: &Covers, stored: Vec<(String, String, String)>) -> Vec<Reduction> {
    let mut reduced: HashMap<String, Option<String>> = HashMap::new();
    stored
        .into_iter()
        .filter_map(|(library, edition, from)| {
            let to = reduced
                .entry(from.clone())
                .or_insert_with(|| covers.reduce_existing(&from).ok().flatten())
                .clone()?;
            Some(Reduction {
                library,
                edition,
                from,
                to,
            })
        })
        .collect()
}

/// Records the new covers, by library and in batches. Only where this device may edit: where it may not, the
/// library is skipped, and the reduced cover will arrive from a device that may. An edition whose cover changed in
/// the meantime keeps the new one.
fn record_reductions(conn: &mut Connection, reductions: &[Reduction]) -> Result<usize> {
    let mut by_library: BTreeMap<&str, Vec<&Reduction>> = BTreeMap::new();
    for reduction in reductions {
        by_library
            .entry(&reduction.library)
            .or_default()
            .push(reduction);
    }
    let mut recorded = 0;
    for (library, reductions) in by_library {
        for batch in reductions.chunks(BATCH) {
            let tx = conn.transaction()?;
            let mut changes = Vec::new();
            for reduction in batch {
                let current: Option<String> = tx
                    .query_row(
                        "SELECT cover FROM editions WHERE library_id = ?1 AND id = ?2",
                        [library, reduction.edition.as_str()],
                        |row| row.get(0),
                    )
                    .optional()?
                    .flatten();
                if current.as_deref() == Some(reduction.from.as_str()) {
                    changes.push(Change::update(
                        Entity::Edition,
                        &reduction.edition,
                        json!({ "cover": reduction.to }),
                    ));
                }
            }
            if changes.is_empty() {
                continue;
            }
            let count = changes.len();
            match ops::record(&tx, library, "edition.reduce_covers", changes) {
                Ok(_) => {
                    tx.commit()?;
                    recorded += count;
                }
                Err(Error::NotAllowed) => break,
                Err(error) => return Err(error),
            }
        }
    }
    Ok(recorded)
}

/// For tests elsewhere (sync.rs): `reduce_stored` on a bare connection.
#[cfg(test)]
pub(crate) fn reduce_stored_in(conn: &mut Connection, covers: &Covers) -> usize {
    let reductions = reduce_files(covers, stored_covers(conn).unwrap());
    record_reductions(conn, &reductions).unwrap()
}

/// Reads a cover in any of the accepted formats and returns it as WebP, at most `MAX_SIDE` on its longest side and
/// never enlarged. Encoding it again leaves out whatever metadata the original carried, such as the place where a
/// phone took a photo.
pub fn normalize(bytes: &[u8]) -> Result<Vec<u8>> {
    let not_an_image = || Error::CoverRejected("not an image");
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| not_an_image())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_INPUT_SIDE);
    limits.max_image_height = Some(MAX_INPUT_SIDE);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| not_an_image())?;
    let image = if image.width() > MAX_SIDE || image.height() > MAX_SIDE {
        image.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Lanczos3)
    } else {
        image
    };
    if image.color().has_alpha() {
        let rgba = image.to_rgba8();
        encode_webp(rgba.as_raw(), rgba.width(), rgba.height(), 4)
    } else {
        let rgb = image.to_rgb8();
        encode_webp(rgb.as_raw(), rgb.width(), rgb.height(), 3)
    }
}

/// libwebp's one-call encoder, with pixels in rows of `width * channels` bytes (RGB or RGBA).
fn encode_webp(pixels: &[u8], width: u32, height: u32, channels: u32) -> Result<Vec<u8>> {
    let failed = || Error::CoverRejected("could not encode");
    let (Ok(columns), Ok(rows), Ok(stride)) = (
        i32::try_from(width),
        i32::try_from(height),
        i32::try_from(width * channels),
    ) else {
        return Err(failed());
    };
    if pixels.len() != (width * channels * height) as usize {
        return Err(failed());
    }
    let mut output: *mut u8 = std::ptr::null_mut();
    // SAFETY: `pixels` holds exactly `rows` rows of `stride` bytes (checked above), which libwebp only reads. It
    // allocates `output` itself and gives back its length; it is freed below with libwebp's own `WebPFree`.
    let length = unsafe {
        if channels == 4 {
            libwebp_sys::WebPEncodeRGBA(
                pixels.as_ptr(),
                columns,
                rows,
                stride,
                QUALITY,
                &mut output,
            )
        } else {
            libwebp_sys::WebPEncodeRGB(pixels.as_ptr(), columns, rows, stride, QUALITY, &mut output)
        }
    };
    if output.is_null() {
        return Err(failed());
    }
    // SAFETY: libwebp wrote `length` bytes at `output`; they are copied out before it is freed, and never used after.
    let encoded = unsafe { std::slice::from_raw_parts(output, length) }.to_vec();
    unsafe { libwebp_sys::WebPFree(output.cast()) };
    if encoded.is_empty() {
        Err(failed())
    } else {
        Ok(encoded)
    }
}

fn extension_for(content_type: &str) -> Option<&'static str> {
    match content_type.split(';').next()?.trim() {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("library-covers-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn stores_by_content_hash_once() {
        let covers = Covers::new(temp_dir("store")).unwrap();

        let first = covers.store(b"image bytes", "jpg").unwrap();
        let second = covers.store(b"image bytes", "jpg").unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 64 + 4);
        assert!(covers.dir().join(&first).exists());
        assert_eq!(
            std::fs::read_dir(covers.dir()).unwrap().count(),
            1,
            "no temporary files left"
        );
    }

    fn encoded(width: u32, height: u32, format: image::ImageFormat) -> Vec<u8> {
        let picture = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([
                u8::try_from(x % 256).unwrap(),
                u8::try_from(y % 256).unwrap(),
                u8::try_from((x + y) % 256).unwrap(),
            ])
        });
        let mut bytes = std::io::Cursor::new(Vec::new());
        picture.write_to(&mut bytes, format).unwrap();
        bytes.into_inner()
    }

    fn webp_size(bytes: &[u8]) -> (u32, u32) {
        let picture = image::load_from_memory_with_format(bytes, image::ImageFormat::WebP).unwrap();
        (picture.width(), picture.height())
    }

    #[test]
    fn a_big_cover_becomes_a_webp_of_at_most_800_pixels_keeping_its_shape() {
        let webp = normalize(&encoded(1200, 1800, image::ImageFormat::Jpeg)).unwrap();

        assert_eq!(&webp[8..12], b"WEBP");
        assert_eq!(webp_size(&webp), (533, 800));
    }

    #[test]
    fn a_small_cover_is_never_enlarged() {
        let webp = normalize(&encoded(300, 450, image::ImageFormat::Png)).unwrap();

        assert_eq!(webp_size(&webp), (300, 450));
    }

    #[test]
    fn what_is_not_an_image_is_refused() {
        assert!(normalize(b"<html>not a cover</html>").is_err());
        assert!(
            normalize(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 16]).is_err(),
            "a cut JPEG"
        );
    }

    #[test]
    fn an_image_too_large_to_be_a_cover_is_refused_before_decoding_it() {
        // A PNG header that claims 20.000 x 20.000 pixels, followed by nothing.
        let mut png = encoded(1, 1, image::ImageFormat::Png);
        png[16..20].copy_from_slice(&20_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&20_000u32.to_be_bytes());

        assert!(normalize(&png).is_err());
    }

    #[test]
    fn the_location_a_phone_wrote_into_a_photo_does_not_survive() {
        let mut jpeg = encoded(400, 600, image::ImageFormat::Jpeg);
        // An APP1 «Exif» segment right after the start-of-image marker, with a recognisable payload.
        let payload = b"Exif\0\0GPS-40.4168-3.7038";
        let length = u16::try_from(payload.len() + 2).unwrap().to_be_bytes();
        let segment: Vec<u8> = [&[0xFF, 0xE1], &length[..], &payload[..]].concat();
        jpeg.splice(2..2, segment);
        assert!(normalize(&jpeg).is_ok(), "the marked photo still reads");

        let webp = normalize(&jpeg).unwrap();

        assert!(!webp.windows(4).any(|window| window == b"Exif"));
        assert!(!webp.windows(3).any(|window| window == b"GPS"));
    }

    /// Run by hand (D82): what real covers weigh as they come, as a JPEG of `MAX_SIDE` and as the WebP that is kept.
    /// `cargo test -p kosmotheka -- --ignored print_cover_sizes --nocapture`
    #[tokio::test]
    #[ignore]
    async fn print_cover_sizes() {
        let covers = [
            (
                "Open Library",
                "https://covers.openlibrary.org/b/isbn/9780140328721-L.jpg",
            ),
            (
                "Open Library",
                "https://covers.openlibrary.org/b/isbn/9780261103252-L.jpg",
            ),
            (
                "Open Library",
                "https://covers.openlibrary.org/b/isbn/9788445000656-L.jpg",
            ),
            (
                "Google Books",
                "https://books.google.com/books/content?id=zyTCAlFPjgYC&printsec=frontcover&img=1&zoom=1",
            ),
            (
                "BoardGameGeek",
                "https://cf.geekdo-images.com/W3Bsga_uLP9kO91gZ7H8yw__original/img/xV7oisd3RQ8R-k18cdWAYthHXsA=/0x0/filters:format(jpeg)/pic2419375.jpg",
            ),
            (
                "BoardGameGeek",
                "https://cf.geekdo-images.com/sZYp_3BTDGjh2unaZfZmuA__original/img/7d-lj5Gd1e8PFnD97LYFah2c45M=/0x0/filters:format(jpeg)/pic2437871.jpg",
            ),
            (
                "TMDB",
                "https://image.tmdb.org/t/p/w500/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg",
            ),
        ];
        let client = reqwest::Client::new();
        let (mut before, mut after) = (0, 0);
        println!("| Source | Original | Size | JPEG 800 q80 | WebP {QUALITY} |");
        println!("|---|---|---|---|---|");
        for (source, url) in covers {
            let bytes = client.get(url).send().await.unwrap().bytes().await.unwrap();
            let picture = image::load_from_memory(&bytes).unwrap();
            let reduced = if picture.width() > MAX_SIDE || picture.height() > MAX_SIDE {
                picture.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Lanczos3)
            } else {
                picture.clone()
            };
            let mut jpeg = std::io::Cursor::new(Vec::new());
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
                .encode_image(&reduced.to_rgb8())
                .unwrap();
            let webp = normalize(&bytes).unwrap();
            before += bytes.len();
            after += webp.len();
            println!(
                "| {source} | {} KB | {}x{} | {} KB | {} KB |",
                bytes.len() / 1024,
                picture.width(),
                picture.height(),
                jpeg.get_ref().len() / 1024,
                webp.len() / 1024
            );
        }
        println!(
            "Total: {} KB as they come, {} KB kept",
            before / 1024,
            after / 1024
        );
    }

    fn book(title: &str) -> kosmotheka_domain::ValidBook {
        kosmotheka_domain::ValidBook {
            edition: kosmotheka_domain::EditionInput {
                title: title.into(),
                ..Default::default()
            }
            .validate()
            .unwrap(),
            copy: kosmotheka_domain::CopyInput {
                owner: Some("Ana".into()),
                format: kosmotheka_domain::Format::Paperback,
                quantity: 1,
                location: None,
                acquired_on: None,
                platform: None,
                seasons: None,
            }
            .validate()
            .unwrap(),
        }
    }

    fn cover_of(conn: &Connection, edition: &str) -> Option<String> {
        conn.query_row(
            "SELECT cover FROM editions WHERE id = ?1",
            [edition],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// Run by hand (D82, part 1 risk): how long the first start of the new version takes to reduce a library of a
    /// thousand books with their own covers, and how many operations it records.
    /// `cargo test -p kosmotheka -- --ignored print_reducing_a_thousand_covers --nocapture`
    #[test]
    #[ignore]
    fn print_reducing_a_thousand_covers() {
        let covers = Covers::new(temp_dir("thousand")).unwrap();
        let mut conn = crate::db::open_in_memory().unwrap();
        let library = ops::active_library(&conn).unwrap();
        for index in 0..1000u32 {
            let picture = image::RgbImage::from_fn(500, 750, |x, y| {
                image::Rgb([
                    u8::try_from((x + index) % 256).unwrap(),
                    u8::try_from((y * 3 + index) % 256).unwrap(),
                    u8::try_from(index % 256).unwrap(),
                ])
            });
            let mut jpeg = std::io::Cursor::new(Vec::new());
            picture
                .write_to(&mut jpeg, image::ImageFormat::Jpeg)
                .unwrap();
            let name = covers.store(jpeg.get_ref(), "jpg").unwrap();
            crate::catalog::create_book(
                &mut conn,
                &library,
                &book(&format!("Book {index}")),
                Some(&name),
            )
            .unwrap();
        }
        let operations_before: i64 = conn
            .query_row("SELECT count(*) FROM operations", [], |row| row.get(0))
            .unwrap();

        let started = std::time::Instant::now();
        let changed = reduce_stored_in(&mut conn, &covers);
        let elapsed = started.elapsed();

        let operations_after: i64 = conn
            .query_row("SELECT count(*) FROM operations", [], |row| row.get(0))
            .unwrap();
        println!(
            "{changed} editions in {:.1} s, {} operations",
            elapsed.as_secs_f64(),
            operations_after - operations_before
        );
    }

    #[test]
    fn a_cover_name_is_a_hash_and_a_known_extension_and_nothing_else() {
        let hash = "a".repeat(64);
        assert!(is_cover_name(&format!("{hash}.webp")));
        assert!(is_cover_name(&format!("{hash}.jpg")));
        assert!(!is_cover_name(&format!("{hash}.exe")));
        assert!(!is_cover_name(&format!("../{}.jpg", "a".repeat(61))));
        assert!(!is_cover_name(&format!("{}.jpg", "A".repeat(64))));
        assert!(!is_cover_name(&format!("{hash}.webp/../x")));
        assert!(!is_cover_name("cover.jpg"));
    }

    #[test]
    fn covers_stored_before_are_reduced_once_and_their_editions_follow() {
        let covers = Covers::new(temp_dir("reduce")).unwrap();
        let mut conn = crate::db::open_in_memory().unwrap();
        let library = ops::active_library(&conn).unwrap();
        let old = covers
            .store(&encoded(1200, 1800, image::ImageFormat::Jpeg), "jpg")
            .unwrap();
        let first = crate::catalog::create_book(&mut conn, &library, &book("Tehanu"), Some(&old))
            .unwrap()
            .edition_id;
        let second = crate::catalog::create_book(
            &mut conn,
            &library,
            &book("Tales from Earthsea"),
            Some(&old),
        )
        .unwrap()
        .edition_id;

        assert_eq!(reduce_stored_in(&mut conn, &covers), 2);

        let reduced = cover_of(&conn, &first).unwrap();
        assert_eq!(
            reduced.rsplit_once('.').map(|(_, extension)| extension),
            Some("webp")
        );
        assert!(covers.is_reduced(&reduced));
        assert_eq!(
            cover_of(&conn, &second),
            Some(reduced),
            "one file, reduced once"
        );
        assert_eq!(
            reduce_stored_in(&mut conn, &covers),
            0,
            "nothing left to do"
        );
    }

    #[test]
    fn a_cover_whose_file_is_not_here_is_left_for_the_device_that_has_it() {
        let covers = Covers::new(temp_dir("absent")).unwrap();
        let mut conn = crate::db::open_in_memory().unwrap();
        let library = ops::active_library(&conn).unwrap();
        let elsewhere = format!("{}.jpg", "b".repeat(64));
        let edition =
            crate::catalog::create_book(&mut conn, &library, &book("Catan"), Some(&elsewhere))
                .unwrap()
                .edition_id;

        assert_eq!(reduce_stored_in(&mut conn, &covers), 0);
        assert_eq!(cover_of(&conn, &edition), Some(elsewhere));
    }

    #[test]
    fn where_this_device_only_reads_its_covers_are_not_changed() {
        let covers = Covers::new(temp_dir("reader")).unwrap();
        let mut reader = crate::db::open_in_memory().unwrap();
        let mut owner = crate::db::open_in_memory().unwrap();
        let games = crate::libraries::create_library(&mut owner, "Juegos").unwrap();
        let old = covers
            .store(&encoded(900, 900, image::ImageFormat::Png), "png")
            .unwrap();
        let edition = crate::catalog::create_book(&mut owner, &games, &book("Catan"), Some(&old))
            .unwrap()
            .edition_id;
        crate::sync::share_as(&owner, &games, &mut reader, "Hugo", "reader");

        assert_eq!(reduce_stored_in(&mut reader, &covers), 0);
        assert_eq!(cover_of(&reader, &edition), Some(old));
    }

    #[test]
    fn known_image_types_only() {
        assert_eq!(extension_for("image/jpeg"), Some("jpg"));
        assert_eq!(extension_for("image/png; charset=binary"), Some("png"));
        assert_eq!(extension_for("text/html"), None);
        assert_eq!(extension_for(""), None);
    }

    #[test]
    fn sha256_matches_a_known_value() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
