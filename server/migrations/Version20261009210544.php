<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261009210544 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The seal (7.4.7, D94): an applied operation that fences the device of its own author fences every device.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE catalog_device_seals (operation_id UUID NOT NULL, until_hlc TEXT NOT NULL COLLATE "C", cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, operation_id))');
        $this->addSql('CREATE INDEX idx_catalog_device_seals_cloud_library ON catalog_device_seals (cloud_library_id)');
        $this->addSql('ALTER TABLE catalog_device_seals ADD CONSTRAINT fk_catalog_device_seals_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        // Derived from the log like the fences: those already noted that fence the device of their own author.
        $this->addSql('INSERT INTO catalog_device_seals (cloud_library_id, operation_id, until_hlc)
            SELECT f.cloud_library_id, f.operation_id, f.until_hlc
              FROM catalog_device_fences f
              JOIN cloud_operation o ON o.cloud_library_id = f.cloud_library_id AND o.operation_id = f.operation_id
             WHERE o.author_device_id = f.device_id');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_device_seals DROP CONSTRAINT fk_catalog_device_seals_cloud_library');
        $this->addSql('DROP TABLE catalog_device_seals');
    }
}
