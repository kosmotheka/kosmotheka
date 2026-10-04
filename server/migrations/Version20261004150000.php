<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/**
 * Hand-written on purpose: these tables have no Doctrine entity (the merge uses
 * DBAL) and doctrine.yaml's schema_filter hides `catalog_*` from the ORM diff.
 */
final class Version20261004150000 extends AbstractMigration
{
    private const array CATALOG_TABLES = [
        'catalog_libraries', 'catalog_works', 'catalog_people', 'catalog_work_contributors', 'catalog_editions',
        'catalog_book_details', 'catalog_comic_details', 'catalog_boardgame_details', 'catalog_film_details',
        'catalog_tv_series_details', 'catalog_members', 'catalog_copies', 'catalog_publishers', 'catalog_tags',
        'catalog_work_tags', 'catalog_work_genres', 'catalog_series', 'catalog_series_entries',
        'catalog_work_relations', 'catalog_revocations', 'catalog_devices', 'catalog_change_requests',
        'catalog_marks', 'catalog_field_versions', 'catalog_tombstones', 'catalog_row_authors',
    ];

    public function getDescription(): string
    {
        return 'Catalog tables of the cloud, keyed by (cloud_library_id, id), with field versions, tombstones and row authors (D41, D84, D89, milestone 7.4.3).';
    }

    public function up(Schema $schema): void
    {
        // Every row belongs to one cloud library and every foreign key between catalog
        // tables carries cloud_library_id, so a row can never point into another library.
        // ids are TEXT as in the app (the operation's string, compared byte by byte).
        // Delete rules (D89): a required reference cascades; an optional one is nulled,
        // only that column (PostgreSQL 15+), never cloud_library_id.
        // The cloud_library_id/cloud_server fields of `library` have no column: the
        // server's own cloud_library.id is the link (D84), never a value from a client.
        $parent = 'cloud_library_id UUID NOT NULL REFERENCES cloud_library (id) ON DELETE CASCADE';

        $this->addSql("CREATE TABLE catalog_libraries ($parent, id TEXT NOT NULL, name TEXT NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql("CREATE TABLE catalog_works ($parent, id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql("CREATE TABLE catalog_people ($parent, id TEXT NOT NULL, name TEXT NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql("CREATE TABLE catalog_publishers ($parent, id TEXT NOT NULL, name TEXT NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql("CREATE TABLE catalog_tags ($parent, id TEXT NOT NULL, name TEXT NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql("CREATE TABLE catalog_series ($parent, id TEXT NOT NULL, name TEXT NOT NULL, ordered BOOLEAN NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql("CREATE TABLE catalog_members (
            $parent, id TEXT NOT NULL, name TEXT NOT NULL,
            role TEXT CHECK (role IS NULL OR role IN ('admin', 'editor', 'reader')),
            can_share BOOLEAN,
            PRIMARY KEY (cloud_library_id, id))");

        $this->addSql("CREATE TABLE catalog_work_contributors (
            $parent, id TEXT NOT NULL, work_id TEXT NOT NULL, person_id TEXT NOT NULL, role TEXT NOT NULL, position BIGINT NOT NULL,
            PRIMARY KEY (cloud_library_id, id),
            UNIQUE (cloud_library_id, work_id, person_id, role),
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, person_id) REFERENCES catalog_people (cloud_library_id, id) ON DELETE CASCADE)");
        $this->addSql('CREATE INDEX idx_catalog_work_contributors_work ON catalog_work_contributors (cloud_library_id, work_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_contributors_person ON catalog_work_contributors (cloud_library_id, person_id)');

        $this->addSql("CREATE TABLE catalog_editions (
            $parent, id TEXT NOT NULL, work_id TEXT NOT NULL, title TEXT NOT NULL, subtitle TEXT, publisher_id TEXT,
            published_year BIGINT, language TEXT, cover TEXT, barcode TEXT, notes TEXT,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, publisher_id) REFERENCES catalog_publishers (cloud_library_id, id) ON DELETE SET NULL (publisher_id))");
        $this->addSql('CREATE INDEX idx_catalog_editions_work ON catalog_editions (cloud_library_id, work_id)');
        $this->addSql('CREATE INDEX idx_catalog_editions_publisher ON catalog_editions (cloud_library_id, publisher_id)');

        // Every kind's details share their edition's id; here that is `id`, not `edition_id` as in the app.
        $details = [
            'book_details' => 'isbn13 TEXT, google_books_id TEXT, open_library_id TEXT',
            'comic_details' => 'number TEXT',
            'boardgame_details' => 'players_min BIGINT, players_max BIGINT, duration_minutes BIGINT, min_age BIGINT',
            'film_details' => 'duration_minutes BIGINT',
            'tv_series_details' => 'seasons BIGINT',
        ];
        foreach ($details as $table => $columns) {
            $this->addSql("CREATE TABLE catalog_$table (
                $parent, id TEXT NOT NULL, $columns,
                PRIMARY KEY (cloud_library_id, id),
                FOREIGN KEY (cloud_library_id, id) REFERENCES catalog_editions (cloud_library_id, id) ON DELETE CASCADE)");
        }

        $this->addSql("CREATE TABLE catalog_copies (
            $parent, id TEXT NOT NULL, edition_id TEXT NOT NULL, owner_id TEXT, format TEXT NOT NULL, quantity BIGINT NOT NULL,
            location TEXT, acquired_on TEXT, platform TEXT, seasons TEXT,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, edition_id) REFERENCES catalog_editions (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, owner_id) REFERENCES catalog_members (cloud_library_id, id) ON DELETE SET NULL (owner_id))");
        $this->addSql('CREATE INDEX idx_catalog_copies_edition ON catalog_copies (cloud_library_id, edition_id)');
        $this->addSql('CREATE INDEX idx_catalog_copies_owner ON catalog_copies (cloud_library_id, owner_id)');

        // Link rows have no unique pair on purpose: two devices may add the same one
        // offline and both rows must survive (D25).
        $this->addSql("CREATE TABLE catalog_work_tags (
            $parent, id TEXT NOT NULL, work_id TEXT NOT NULL, tag_id TEXT NOT NULL,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, tag_id) REFERENCES catalog_tags (cloud_library_id, id) ON DELETE CASCADE)");
        $this->addSql('CREATE INDEX idx_catalog_work_tags_work ON catalog_work_tags (cloud_library_id, work_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_tags_tag ON catalog_work_tags (cloud_library_id, tag_id)');

        $this->addSql("CREATE TABLE catalog_work_genres (
            $parent, id TEXT NOT NULL, work_id TEXT NOT NULL, genre TEXT NOT NULL,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE)");
        $this->addSql('CREATE INDEX idx_catalog_work_genres_work ON catalog_work_genres (cloud_library_id, work_id)');

        $this->addSql("CREATE TABLE catalog_series_entries (
            $parent, id TEXT NOT NULL, series_id TEXT NOT NULL, work_id TEXT NOT NULL, position BIGINT,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, series_id) REFERENCES catalog_series (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE)");
        $this->addSql('CREATE INDEX idx_catalog_series_entries_series ON catalog_series_entries (cloud_library_id, series_id)');
        $this->addSql('CREATE INDEX idx_catalog_series_entries_work ON catalog_series_entries (cloud_library_id, work_id)');

        $this->addSql("CREATE TABLE catalog_work_relations (
            $parent, id TEXT NOT NULL, work_id TEXT NOT NULL, related_work_id TEXT NOT NULL, relation TEXT NOT NULL,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, related_work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE)");
        $this->addSql('CREATE INDEX idx_catalog_work_relations_work ON catalog_work_relations (cloud_library_id, work_id)');
        $this->addSql('CREATE INDEX idx_catalog_work_relations_related ON catalog_work_relations (cloud_library_id, related_work_id)');

        $this->addSql("CREATE TABLE catalog_marks (
            $parent, id TEXT NOT NULL, member_id TEXT NOT NULL, work_id TEXT NOT NULL,
            status TEXT CHECK (status IS NULL OR status IN ('pending', 'in_progress', 'finished', 'abandoned')),
            status_on TEXT, rating INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 5),
            favorite BOOLEAN, wished BOOLEAN,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, work_id) REFERENCES catalog_works (cloud_library_id, id) ON DELETE CASCADE,
            FOREIGN KEY (cloud_library_id, member_id) REFERENCES catalog_members (cloud_library_id, id) ON DELETE CASCADE)");
        $this->addSql('CREATE INDEX idx_catalog_marks_work ON catalog_marks (cloud_library_id, work_id)');
        $this->addSql('CREATE INDEX idx_catalog_marks_member ON catalog_marks (cloud_library_id, member_id)');

        $this->addSql("CREATE TABLE catalog_revocations ($parent, id TEXT NOT NULL, device_id TEXT NOT NULL, device_name TEXT NOT NULL, PRIMARY KEY (cloud_library_id, id))");
        $this->addSql('CREATE INDEX idx_catalog_revocations_device ON catalog_revocations (cloud_library_id, device_id)');

        $this->addSql("CREATE TABLE catalog_devices (
            $parent, id TEXT NOT NULL, public_key TEXT NOT NULL, name TEXT NOT NULL, member_id TEXT, trusted_through TEXT,
            PRIMARY KEY (cloud_library_id, id),
            FOREIGN KEY (cloud_library_id, member_id) REFERENCES catalog_members (cloud_library_id, id) ON DELETE SET NULL (member_id))");
        $this->addSql('CREATE INDEX idx_catalog_devices_member ON catalog_devices (cloud_library_id, member_id)');

        $this->addSql("CREATE TABLE catalog_change_requests ($parent, id TEXT NOT NULL,
            status TEXT CHECK (status IS NULL OR status IN ('accepted', 'rejected', 'withdrawn')),
            note TEXT, PRIMARY KEY (cloud_library_id, id))");

        // Bookkeeping of the merge and the permissions. `entity` is the CatalogEntity value
        // ('work'), not the app's table name ('works'). HLCs compare byte by byte (collation C),
        // as in cloud_operation. No foreign keys: a version or a tombstone outlives its row.
        $this->addSql("CREATE TABLE catalog_field_versions (
            $parent, entity TEXT NOT NULL, entity_id TEXT NOT NULL, field TEXT NOT NULL, hlc TEXT NOT NULL COLLATE \"C\",
            PRIMARY KEY (cloud_library_id, entity, entity_id, field))");
        $this->addSql("CREATE TABLE catalog_tombstones (
            $parent, entity TEXT NOT NULL, entity_id TEXT NOT NULL, hlc TEXT NOT NULL COLLATE \"C\",
            PRIMARY KEY (cloud_library_id, entity, entity_id))");
        $this->addSql("CREATE TABLE catalog_row_authors (
            $parent, entity TEXT NOT NULL, entity_id TEXT NOT NULL, device_id TEXT NOT NULL,
            PRIMARY KEY (cloud_library_id, entity, entity_id))");
    }

    public function down(Schema $schema): void
    {
        // One statement: the tables reference each other, and only the whole set can go.
        $this->addSql('DROP TABLE '.implode(', ', self::CATALOG_TABLES));
    }
}
