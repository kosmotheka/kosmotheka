<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261015214624 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'D97: a library is founded by one key (founding_nonce); the seal of D94 is gone';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_device_seals DROP CONSTRAINT fk_catalog_device_seals_cloud_library');
        $this->addSql('DROP TABLE catalog_device_seals');
        $this->addSql('ALTER TABLE catalog_libraries ADD founding_nonce TEXT DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('CREATE TABLE catalog_device_seals (operation_id UUID NOT NULL, until_hlc TEXT NOT NULL COLLATE "C", cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, operation_id))');
        $this->addSql('CREATE INDEX idx_catalog_device_seals_cloud_library ON catalog_device_seals (cloud_library_id)');
        $this->addSql('ALTER TABLE catalog_device_seals ADD CONSTRAINT fk_catalog_device_seals_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE INITIALLY IMMEDIATE');
        $this->addSql('ALTER TABLE catalog_libraries DROP founding_nonce');
    }
}
