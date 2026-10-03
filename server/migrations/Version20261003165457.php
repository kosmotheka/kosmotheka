<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261003165457 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Cloud libraries, their access table, operation log and device keys (D84, milestone 7.2).';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE cloud_library (id UUID NOT NULL, local_library_id VARCHAR(36) DEFAULT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, owner_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_cloud_library_owner ON cloud_library (owner_id)');
        $this->addSql('ALTER TABLE cloud_library ADD CONSTRAINT fk_cloud_library_owner FOREIGN KEY (owner_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');

        $this->addSql('CREATE TABLE membership (id UUID NOT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, member_id UUID DEFAULT NULL, user_id UUID NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_membership_user ON membership (user_id)');
        $this->addSql('CREATE INDEX idx_membership_cloud_library ON membership (cloud_library_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_membership_user_cloud_library ON membership (user_id, cloud_library_id)');
        $this->addSql('ALTER TABLE membership ADD CONSTRAINT fk_membership_user FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE membership ADD CONSTRAINT fk_membership_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');

        // Operation ids are unique inside a cloud library, never across them (seam 1).
        // The HLC compares byte by byte (collation C): fixed width, so that is HLC order.
        $this->addSql('CREATE TABLE cloud_operation (operation_id UUID NOT NULL, author_device_id VARCHAR(36) NOT NULL, hlc VARCHAR(255) NOT NULL COLLATE "C", schema_version INT NOT NULL, body JSONB NOT NULL, received_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, operation_id))');
        $this->addSql('CREATE INDEX idx_cloud_operation_cloud_library ON cloud_operation (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_cloud_operation_author_hlc ON cloud_operation (cloud_library_id, author_device_id, hlc)');
        $this->addSql('ALTER TABLE cloud_operation ADD CONSTRAINT fk_cloud_operation_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');

        $this->addSql('CREATE TABLE cloud_device_key (device_id VARCHAR(36) NOT NULL, public_key VARCHAR(43) NOT NULL, cloud_library_id UUID NOT NULL, PRIMARY KEY (cloud_library_id, device_id))');
        $this->addSql('CREATE INDEX idx_cloud_device_key_cloud_library ON cloud_device_key (cloud_library_id)');
        $this->addSql('ALTER TABLE cloud_device_key ADD CONSTRAINT fk_cloud_device_key_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP TABLE cloud_device_key');
        $this->addSql('DROP TABLE cloud_operation');
        $this->addSql('DROP TABLE membership');
        $this->addSql('DROP TABLE cloud_library');
    }
}
