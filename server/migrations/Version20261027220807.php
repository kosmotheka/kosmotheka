<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261027220807 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'When the server first saw each row in the trash of a cloud library (D105), for its 60 days';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE catalog_trash_seen (entity TEXT NOT NULL, entity_id TEXT NOT NULL, seen_at BIGINT NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, entity, entity_id))');
        $this->addSql('CREATE INDEX idx_catalog_trash_seen_cloud_library ON catalog_trash_seen (cloud_library_id)');
        $this->addSql('ALTER TABLE catalog_trash_seen ADD CONSTRAINT fk_catalog_trash_seen_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_trash_seen DROP CONSTRAINT fk_catalog_trash_seen_cloud_library');
        $this->addSql('DROP TABLE catalog_trash_seen');
    }
}
