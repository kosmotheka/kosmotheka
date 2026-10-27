<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261027213609 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The trash (D105): trashed_at and trashed_by on editions, copies, tags and series';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_copies ADD trashed_at TEXT DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_copies ADD trashed_by_key UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_copies ADD CONSTRAINT fk_catalog_copies_trashed_by_key FOREIGN KEY (trashed_by_key) REFERENCES catalog_members (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('CREATE INDEX idx_catalog_copies_trashed_by_key ON catalog_copies (trashed_by_key)');
        $this->addSql('ALTER TABLE catalog_editions ADD trashed_at TEXT DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_editions ADD trashed_by_key UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_editions ADD CONSTRAINT fk_catalog_editions_trashed_by_key FOREIGN KEY (trashed_by_key) REFERENCES catalog_members (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('CREATE INDEX idx_catalog_editions_trashed_by_key ON catalog_editions (trashed_by_key)');
        $this->addSql('ALTER TABLE catalog_series ADD trashed_at TEXT DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_series ADD trashed_by_key UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_series ADD CONSTRAINT fk_catalog_series_trashed_by_key FOREIGN KEY (trashed_by_key) REFERENCES catalog_members (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('CREATE INDEX idx_catalog_series_trashed_by_key ON catalog_series (trashed_by_key)');
        $this->addSql('ALTER TABLE catalog_tags ADD trashed_at TEXT DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_tags ADD trashed_by_key UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_tags ADD CONSTRAINT fk_catalog_tags_trashed_by_key FOREIGN KEY (trashed_by_key) REFERENCES catalog_members (key) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('CREATE INDEX idx_catalog_tags_trashed_by_key ON catalog_tags (trashed_by_key)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_copies DROP CONSTRAINT fk_catalog_copies_trashed_by_key');
        $this->addSql('DROP INDEX idx_catalog_copies_trashed_by_key');
        $this->addSql('ALTER TABLE catalog_copies DROP trashed_at');
        $this->addSql('ALTER TABLE catalog_copies DROP trashed_by_key');
        $this->addSql('ALTER TABLE catalog_editions DROP CONSTRAINT fk_catalog_editions_trashed_by_key');
        $this->addSql('DROP INDEX idx_catalog_editions_trashed_by_key');
        $this->addSql('ALTER TABLE catalog_editions DROP trashed_at');
        $this->addSql('ALTER TABLE catalog_editions DROP trashed_by_key');
        $this->addSql('ALTER TABLE catalog_series DROP CONSTRAINT fk_catalog_series_trashed_by_key');
        $this->addSql('DROP INDEX idx_catalog_series_trashed_by_key');
        $this->addSql('ALTER TABLE catalog_series DROP trashed_at');
        $this->addSql('ALTER TABLE catalog_series DROP trashed_by_key');
        $this->addSql('ALTER TABLE catalog_tags DROP CONSTRAINT fk_catalog_tags_trashed_by_key');
        $this->addSql('DROP INDEX idx_catalog_tags_trashed_by_key');
        $this->addSql('ALTER TABLE catalog_tags DROP trashed_at');
        $this->addSql('ALTER TABLE catalog_tags DROP trashed_by_key');
    }
}
