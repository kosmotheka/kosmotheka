-- Cover file name (sha256 + extension) and ids of the edition in external catalogues.
ALTER TABLE editions ADD COLUMN cover TEXT;
ALTER TABLE book_details ADD COLUMN google_books_id TEXT;
ALTER TABLE book_details ADD COLUMN open_library_id TEXT;
