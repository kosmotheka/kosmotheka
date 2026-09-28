-- Adds the "wished" flag to marks (D68/D61): "I wish I had it", used for gifts.
ALTER TABLE marks ADD COLUMN wished INTEGER CHECK (wished IS NULL OR wished IN (0, 1));
