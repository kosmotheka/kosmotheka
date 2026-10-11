<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261011144044 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Metadata cache: what a provider said about one ISBN, with nothing about who asked (7.8)';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE metadata_cache (id UUID NOT NULL, provider VARCHAR(32) NOT NULL, code VARCHAR(13) NOT NULL, found BOOLEAN NOT NULL, payload JSON NOT NULL, fetched_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_metadata_cache_fetched ON metadata_cache (fetched_at)');
        $this->addSql('CREATE UNIQUE INDEX uniq_metadata_cache_provider_code ON metadata_cache (provider, code)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP TABLE metadata_cache');
    }
}
