-- The barcode printed on the edition (EAN or UPC): games, discs and films have no
-- ISBN, and the scanner looks them up by this. Books keep their ISBN in book_details.
ALTER TABLE editions ADD COLUMN barcode TEXT;
CREATE INDEX editions_barcode ON editions (library_id, barcode);
