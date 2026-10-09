<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261009111215 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The permission rules on the server (7.4.5): the status and previous of an operation of the log, and the tables of fences.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE catalog_device_fence_chains (fence_operation_id UUID NOT NULL, device_id TEXT NOT NULL, operation_id UUID NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, fence_operation_id, device_id, operation_id))');
        $this->addSql('CREATE INDEX idx_catalog_device_fence_chains_cloud_library ON catalog_device_fence_chains (cloud_library_id)');
        $this->addSql('CREATE TABLE catalog_device_fences (operation_id UUID NOT NULL, device_id TEXT NOT NULL, through TEXT NOT NULL, until_hlc TEXT NOT NULL COLLATE "C", cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, operation_id, device_id))');
        $this->addSql('CREATE INDEX idx_catalog_device_fences_cloud_library ON catalog_device_fences (cloud_library_id)');
        $this->addSql('ALTER TABLE catalog_device_fence_chains ADD CONSTRAINT fk_catalog_device_fence_chains_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE catalog_device_fences ADD CONSTRAINT fk_catalog_device_fences_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE cloud_operation ADD previous UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE cloud_operation ADD status VARCHAR(16) DEFAULT NULL');
        // The operations already stored keep the `previous` they came with (the body is what the signature covers).
        $this->addSql("UPDATE cloud_operation SET previous = (body ->> 'previous')::uuid WHERE body ->> 'previous' IS NOT NULL");
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_device_fence_chains DROP CONSTRAINT fk_catalog_device_fence_chains_cloud_library');
        $this->addSql('ALTER TABLE catalog_device_fences DROP CONSTRAINT fk_catalog_device_fences_cloud_library');
        $this->addSql('DROP TABLE catalog_device_fence_chains');
        $this->addSql('DROP TABLE catalog_device_fences');
        $this->addSql('ALTER TABLE cloud_operation DROP previous');
        $this->addSql('ALTER TABLE cloud_operation DROP status');
    }
}
