<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261024154322 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Cover files of the cloud libraries (D82): one row per file the server keeps, for the limit per library and the cleanup';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE cloud_cover (id UUID NOT NULL, name VARCHAR(69) NOT NULL, size INT NOT NULL, stored_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_cloud_cover_cloud_library ON cloud_cover (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_cloud_cover_stored_at ON cloud_cover (stored_at)');
        $this->addSql('CREATE UNIQUE INDEX uniq_cloud_cover_library_name ON cloud_cover (cloud_library_id, name)');
        $this->addSql('ALTER TABLE cloud_cover ADD CONSTRAINT fk_cloud_cover_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_cover DROP CONSTRAINT fk_cloud_cover_cloud_library');
        $this->addSql('DROP TABLE cloud_cover');
    }
}
