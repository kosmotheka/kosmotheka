-- A video game's platform and a series' held seasons belong to each copy, not the
-- edition (one Zelda can have a Switch copy and a Wii U copy): platform moves from
-- videogame_details to copies, and seasons (free text) is new on copies.
ALTER TABLE copies ADD COLUMN platform TEXT;
ALTER TABLE copies ADD COLUMN seasons TEXT;

UPDATE copies
   SET platform = (
        SELECT platform FROM videogame_details v
         WHERE v.library_id = copies.library_id AND v.edition_id = copies.edition_id
       )
 WHERE platform IS NULL;

DROP TABLE videogame_details;
