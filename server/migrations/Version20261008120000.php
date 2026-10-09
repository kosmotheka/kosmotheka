<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261008120000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Catalog tables as Doctrine entities with a key of the server\'s own, replacing the hand-written ones of 7.4.3 (D92).';
    }

    public function up(Schema $schema): void
    {
        // The tables of 7.4.3 were never read or written by anyone: they are replaced, not migrated.
        $this->addSql('DROP TABLE IF EXISTS catalog_libraries, catalog_works, catalog_people, catalog_work_contributors, catalog_editions, catalog_book_details, catalog_comic_details, catalog_boardgame_details, catalog_film_details, catalog_tv_series_details, catalog_members, catalog_copies, catalog_publishers, catalog_tags, catalog_work_tags, catalog_work_genres, catalog_series, catalog_series_entries, catalog_work_relations, catalog_revocations, catalog_devices, catalog_change_requests, catalog_marks, catalog_field_versions, catalog_tombstones, catalog_row_authors');

        // The SQL below is what Doctrine generates for the entities in src/Entity/Catalog/.
        $this->addSql('CREATE TABLE catalog_boardgame_details (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, players_min BIGINT DEFAULT NULL, players_max BIGINT DEFAULT NULL, duration_minutes BIGINT DEFAULT NULL, min_age BIGINT DEFAULT NULL, cloud_library_id UUID NOT NULL, edition_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_boardgame_details_cloud_library ON catalog_boardgame_details (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_boardgame_details_edition_key ON catalog_boardgame_details (edition_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_boardgame_details_library_id ON catalog_boardgame_details (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_book_details (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, isbn13 TEXT DEFAULT NULL, google_books_id TEXT DEFAULT NULL, open_library_id TEXT DEFAULT NULL, cloud_library_id UUID NOT NULL, edition_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_book_details_cloud_library ON catalog_book_details (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_book_details_edition_key ON catalog_book_details (edition_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_book_details_library_id ON catalog_book_details (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_change_requests (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, status TEXT DEFAULT NULL, note TEXT DEFAULT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_change_requests_cloud_library ON catalog_change_requests (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_change_requests_library_id ON catalog_change_requests (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_comic_details (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, number TEXT DEFAULT NULL, cloud_library_id UUID NOT NULL, edition_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_comic_details_cloud_library ON catalog_comic_details (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_comic_details_edition_key ON catalog_comic_details (edition_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_comic_details_library_id ON catalog_comic_details (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_copies (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, format TEXT NOT NULL, quantity BIGINT NOT NULL, location TEXT DEFAULT NULL, acquired_on TEXT DEFAULT NULL, platform TEXT DEFAULT NULL, seasons TEXT DEFAULT NULL, cloud_library_id UUID NOT NULL, edition_key UUID NOT NULL, owner_key UUID DEFAULT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_copies_cloud_library ON catalog_copies (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_copies_edition_key ON catalog_copies (edition_key)');
        $this->addSql('CREATE INDEX idx_catalog_copies_owner_key ON catalog_copies (owner_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_copies_library_id ON catalog_copies (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_devices (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, public_key TEXT NOT NULL, name TEXT NOT NULL, trusted_through TEXT DEFAULT NULL, cloud_library_id UUID NOT NULL, member_key UUID DEFAULT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_devices_cloud_library ON catalog_devices (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_devices_member_key ON catalog_devices (member_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_devices_library_id ON catalog_devices (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_editions (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, title TEXT NOT NULL, subtitle TEXT DEFAULT NULL, published_year BIGINT DEFAULT NULL, language TEXT DEFAULT NULL, cover TEXT DEFAULT NULL, barcode TEXT DEFAULT NULL, notes TEXT DEFAULT NULL, cloud_library_id UUID NOT NULL, work_key UUID NOT NULL, publisher_key UUID DEFAULT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_editions_cloud_library ON catalog_editions (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_editions_work_key ON catalog_editions (work_key)');
        $this->addSql('CREATE INDEX idx_catalog_editions_publisher_key ON catalog_editions (publisher_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_editions_library_id ON catalog_editions (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_film_details (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, duration_minutes BIGINT DEFAULT NULL, cloud_library_id UUID NOT NULL, edition_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_film_details_cloud_library ON catalog_film_details (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_film_details_edition_key ON catalog_film_details (edition_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_film_details_library_id ON catalog_film_details (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_libraries (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_libraries_cloud_library ON catalog_libraries (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_libraries_library_id ON catalog_libraries (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_marks (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, status TEXT DEFAULT NULL, status_on TEXT DEFAULT NULL, rating INT DEFAULT NULL, favorite BOOLEAN DEFAULT NULL, wished BOOLEAN DEFAULT NULL, cloud_library_id UUID NOT NULL, member_key UUID NOT NULL, work_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_marks_cloud_library ON catalog_marks (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_marks_member_key ON catalog_marks (member_key)');
        $this->addSql('CREATE INDEX idx_catalog_marks_work_key ON catalog_marks (work_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_marks_library_id ON catalog_marks (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_members (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, role TEXT DEFAULT NULL, can_share BOOLEAN DEFAULT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_members_cloud_library ON catalog_members (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_members_library_id ON catalog_members (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_people (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_people_cloud_library ON catalog_people (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_people_library_id ON catalog_people (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_publishers (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_publishers_cloud_library ON catalog_publishers (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_publishers_library_id ON catalog_publishers (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_revocations (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, device_id TEXT NOT NULL, device_name TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_revocations_cloud_library ON catalog_revocations (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_revocations_library_id ON catalog_revocations (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_row_authors (entity TEXT NOT NULL, entity_id TEXT NOT NULL, device_id TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, entity, entity_id))');
        $this->addSql('CREATE INDEX idx_catalog_row_authors_cloud_library ON catalog_row_authors (cloud_library_id)');
        $this->addSql('CREATE TABLE catalog_series (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, ordered BOOLEAN NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_series_cloud_library ON catalog_series (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_series_library_id ON catalog_series (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_series_entries (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, position BIGINT DEFAULT NULL, cloud_library_id UUID NOT NULL, series_key UUID NOT NULL, work_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_series_entries_cloud_library ON catalog_series_entries (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_series_entries_series_key ON catalog_series_entries (series_key)');
        $this->addSql('CREATE INDEX idx_catalog_series_entries_work_key ON catalog_series_entries (work_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_series_entries_library_id ON catalog_series_entries (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_tags (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, name TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_tags_cloud_library ON catalog_tags (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_tags_library_id ON catalog_tags (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_tombstones (entity TEXT NOT NULL, entity_id TEXT NOT NULL, hlc TEXT NOT NULL COLLATE "C", cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, entity, entity_id))');
        $this->addSql('CREATE INDEX idx_catalog_tombstones_cloud_library ON catalog_tombstones (cloud_library_id)');
        $this->addSql('CREATE TABLE catalog_tv_series_details (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, seasons BIGINT DEFAULT NULL, cloud_library_id UUID NOT NULL, edition_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_tv_series_details_cloud_library ON catalog_tv_series_details (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_tv_series_details_edition_key ON catalog_tv_series_details (edition_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_tv_series_details_library_id ON catalog_tv_series_details (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_works (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_works_cloud_library ON catalog_works (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_works_library_id ON catalog_works (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_work_contributors (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, role TEXT NOT NULL, position BIGINT NOT NULL, cloud_library_id UUID NOT NULL, work_key UUID NOT NULL, person_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_work_contributors_cloud_library ON catalog_work_contributors (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_contributors_work_key ON catalog_work_contributors (work_key)');
        $this->addSql('CREATE INDEX idx_catalog_work_contributors_person_key ON catalog_work_contributors (person_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_work_contributors_library_id ON catalog_work_contributors (cloud_library_id, id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_work_contributors_work_person_role ON catalog_work_contributors (work_key, person_key, role)');
        $this->addSql('CREATE TABLE catalog_work_genres (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, genre TEXT NOT NULL, cloud_library_id UUID NOT NULL, work_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_work_genres_cloud_library ON catalog_work_genres (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_genres_work_key ON catalog_work_genres (work_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_work_genres_library_id ON catalog_work_genres (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_work_relations (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, relation TEXT NOT NULL, cloud_library_id UUID NOT NULL, work_key UUID NOT NULL, related_work_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_work_relations_cloud_library ON catalog_work_relations (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_relations_work_key ON catalog_work_relations (work_key)');
        $this->addSql('CREATE INDEX idx_catalog_work_relations_related_work_key ON catalog_work_relations (related_work_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_work_relations_library_id ON catalog_work_relations (cloud_library_id, id)');
        $this->addSql('CREATE TABLE catalog_work_tags (key UUID NOT NULL, versions JSONB NOT NULL, id TEXT NOT NULL, cloud_library_id UUID NOT NULL, work_key UUID NOT NULL, tag_key UUID NOT NULL, PRIMARY KEY (key))');
        $this->addSql('CREATE INDEX idx_catalog_work_tags_cloud_library ON catalog_work_tags (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_tags_work_key ON catalog_work_tags (work_key)');
        $this->addSql('CREATE INDEX idx_catalog_work_tags_tag_key ON catalog_work_tags (tag_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_catalog_work_tags_library_id ON catalog_work_tags (cloud_library_id, id)');
        $this->addSql('ALTER TABLE catalog_boardgame_details ADD CONSTRAINT fk_catalog_boardgame_details_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_boardgame_details ADD CONSTRAINT fk_catalog_boardgame_details_edition_key FOREIGN KEY (edition_key) REFERENCES catalog_editions (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_book_details ADD CONSTRAINT fk_catalog_book_details_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_book_details ADD CONSTRAINT fk_catalog_book_details_edition_key FOREIGN KEY (edition_key) REFERENCES catalog_editions (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_change_requests ADD CONSTRAINT fk_catalog_change_requests_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_comic_details ADD CONSTRAINT fk_catalog_comic_details_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_comic_details ADD CONSTRAINT fk_catalog_comic_details_edition_key FOREIGN KEY (edition_key) REFERENCES catalog_editions (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_copies ADD CONSTRAINT fk_catalog_copies_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_copies ADD CONSTRAINT fk_catalog_copies_edition_key FOREIGN KEY (edition_key) REFERENCES catalog_editions (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_copies ADD CONSTRAINT fk_catalog_copies_owner_key FOREIGN KEY (owner_key) REFERENCES catalog_members (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_devices ADD CONSTRAINT fk_catalog_devices_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_devices ADD CONSTRAINT fk_catalog_devices_member_key FOREIGN KEY (member_key) REFERENCES catalog_members (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_editions ADD CONSTRAINT fk_catalog_editions_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_editions ADD CONSTRAINT fk_catalog_editions_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_editions ADD CONSTRAINT fk_catalog_editions_publisher_key FOREIGN KEY (publisher_key) REFERENCES catalog_publishers (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_film_details ADD CONSTRAINT fk_catalog_film_details_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_film_details ADD CONSTRAINT fk_catalog_film_details_edition_key FOREIGN KEY (edition_key) REFERENCES catalog_editions (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_libraries ADD CONSTRAINT fk_catalog_libraries_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_marks ADD CONSTRAINT fk_catalog_marks_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_marks ADD CONSTRAINT fk_catalog_marks_member_key FOREIGN KEY (member_key) REFERENCES catalog_members (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_marks ADD CONSTRAINT fk_catalog_marks_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_members ADD CONSTRAINT fk_catalog_members_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_people ADD CONSTRAINT fk_catalog_people_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_publishers ADD CONSTRAINT fk_catalog_publishers_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_revocations ADD CONSTRAINT fk_catalog_revocations_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_row_authors ADD CONSTRAINT fk_catalog_row_authors_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_series ADD CONSTRAINT fk_catalog_series_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_series_entries ADD CONSTRAINT fk_catalog_series_entries_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_series_entries ADD CONSTRAINT fk_catalog_series_entries_series_key FOREIGN KEY (series_key) REFERENCES catalog_series (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_series_entries ADD CONSTRAINT fk_catalog_series_entries_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_tags ADD CONSTRAINT fk_catalog_tags_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_tombstones ADD CONSTRAINT fk_catalog_tombstones_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_tv_series_details ADD CONSTRAINT fk_catalog_tv_series_details_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_tv_series_details ADD CONSTRAINT fk_catalog_tv_series_details_edition_key FOREIGN KEY (edition_key) REFERENCES catalog_editions (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_works ADD CONSTRAINT fk_catalog_works_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_contributors ADD CONSTRAINT fk_catalog_work_contributors_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_contributors ADD CONSTRAINT fk_catalog_work_contributors_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_contributors ADD CONSTRAINT fk_catalog_work_contributors_person_key FOREIGN KEY (person_key) REFERENCES catalog_people (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_genres ADD CONSTRAINT fk_catalog_work_genres_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_genres ADD CONSTRAINT fk_catalog_work_genres_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_relations ADD CONSTRAINT fk_catalog_work_relations_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_relations ADD CONSTRAINT fk_catalog_work_relations_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_relations ADD CONSTRAINT fk_catalog_work_relations_related_work_key FOREIGN KEY (related_work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_tags ADD CONSTRAINT fk_catalog_work_tags_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_tags ADD CONSTRAINT fk_catalog_work_tags_work_key FOREIGN KEY (work_key) REFERENCES catalog_works (key) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_work_tags ADD CONSTRAINT fk_catalog_work_tags_tag_key FOREIGN KEY (tag_key) REFERENCES catalog_tags (key) ON DELETE CASCADE NOT DEFERRABLE');

        // --- Same-library triggers (D92) ---------------------------------------------------
        // Foreign keys are one column now (to `key`), so the database no longer stops a row
        // from pointing into another cloud library by itself. These triggers do: one per
        // reference, plus the `edition_key` of every details table. They are the safety net
        // behind the merge engine, which resolves ids inside the library and never needs them.
        // `make migration` does not see triggers: this block is written by hand.
        $this->addSql(<<<'SQL'
            CREATE FUNCTION catalog_same_library() RETURNS trigger LANGUAGE plpgsql AS $$
            DECLARE
                target uuid;
            BEGIN
                -- TG_ARGV[0] is the referenced table, TG_ARGV[1] the column holding the reference.
                EXECUTE format('SELECT cloud_library_id FROM %I WHERE key = $1', TG_ARGV[0])
                    INTO target USING (to_jsonb(NEW) ->> TG_ARGV[1])::uuid;
                IF target IS NOT NULL AND target <> NEW.cloud_library_id THEN
                    RAISE EXCEPTION 'row of library % points into library % through %', NEW.cloud_library_id, target, TG_ARGV[1]
                        USING ERRCODE = '23503';
                END IF;

                RETURN NEW;
            END
            $$
            SQL);
        $this->addSql('CREATE TRIGGER catalog_work_contributors_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_contributors FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
        $this->addSql('CREATE TRIGGER catalog_work_contributors_person_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_contributors FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_people\', \'person_key\')');
        $this->addSql('CREATE TRIGGER catalog_editions_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_editions FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
        $this->addSql('CREATE TRIGGER catalog_editions_publisher_key_same_library BEFORE INSERT OR UPDATE ON catalog_editions FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_publishers\', \'publisher_key\')');
        $this->addSql('CREATE TRIGGER catalog_book_details_edition_key_same_library BEFORE INSERT OR UPDATE ON catalog_book_details FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_editions\', \'edition_key\')');
        $this->addSql('CREATE TRIGGER catalog_comic_details_edition_key_same_library BEFORE INSERT OR UPDATE ON catalog_comic_details FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_editions\', \'edition_key\')');
        $this->addSql('CREATE TRIGGER catalog_boardgame_details_edition_key_same_library BEFORE INSERT OR UPDATE ON catalog_boardgame_details FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_editions\', \'edition_key\')');
        $this->addSql('CREATE TRIGGER catalog_film_details_edition_key_same_library BEFORE INSERT OR UPDATE ON catalog_film_details FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_editions\', \'edition_key\')');
        $this->addSql('CREATE TRIGGER catalog_tv_series_details_edition_key_same_library BEFORE INSERT OR UPDATE ON catalog_tv_series_details FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_editions\', \'edition_key\')');
        $this->addSql('CREATE TRIGGER catalog_copies_edition_key_same_library BEFORE INSERT OR UPDATE ON catalog_copies FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_editions\', \'edition_key\')');
        $this->addSql('CREATE TRIGGER catalog_copies_owner_key_same_library BEFORE INSERT OR UPDATE ON catalog_copies FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_members\', \'owner_key\')');
        $this->addSql('CREATE TRIGGER catalog_work_tags_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_tags FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
        $this->addSql('CREATE TRIGGER catalog_work_tags_tag_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_tags FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_tags\', \'tag_key\')');
        $this->addSql('CREATE TRIGGER catalog_work_genres_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_genres FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
        $this->addSql('CREATE TRIGGER catalog_series_entries_series_key_same_library BEFORE INSERT OR UPDATE ON catalog_series_entries FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_series\', \'series_key\')');
        $this->addSql('CREATE TRIGGER catalog_series_entries_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_series_entries FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
        $this->addSql('CREATE TRIGGER catalog_work_relations_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_relations FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
        $this->addSql('CREATE TRIGGER catalog_work_relations_related_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_work_relations FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'related_work_key\')');
        $this->addSql('CREATE TRIGGER catalog_devices_member_key_same_library BEFORE INSERT OR UPDATE ON catalog_devices FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_members\', \'member_key\')');
        $this->addSql('CREATE TRIGGER catalog_marks_member_key_same_library BEFORE INSERT OR UPDATE ON catalog_marks FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_members\', \'member_key\')');
        $this->addSql('CREATE TRIGGER catalog_marks_work_key_same_library BEFORE INSERT OR UPDATE ON catalog_marks FOR EACH ROW EXECUTE FUNCTION catalog_same_library(\'catalog_works\', \'work_key\')');
    }

    public function down(Schema $schema): void
    {
        // Back to no catalog tables at all: the hand-written ones of 7.4.3 were used by nobody and are not rebuilt.
        $this->addSql('DROP FUNCTION IF EXISTS catalog_same_library() CASCADE');
        $this->addSql('DROP TABLE catalog_boardgame_details, catalog_book_details, catalog_change_requests, catalog_comic_details, catalog_copies, catalog_devices, catalog_editions, catalog_film_details, catalog_libraries, catalog_marks, catalog_members, catalog_people, catalog_publishers, catalog_revocations, catalog_row_authors, catalog_series, catalog_series_entries, catalog_tags, catalog_tombstones, catalog_tv_series_details, catalog_work_contributors, catalog_work_genres, catalog_work_relations, catalog_work_tags, catalog_works');
    }
}
