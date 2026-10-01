<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/** Devices and their sign-in tokens (milestone 7.1, part 2). */
final class Version20261002000000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Create the device table for signing a device into a server account';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE device (id VARCHAR(36) NOT NULL, name VARCHAR(100) NOT NULL, public_key VARCHAR(43) NOT NULL, token_hash VARCHAR(64) DEFAULT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, last_seen_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, user_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_device_user ON device (user_id)');
        $this->addSql('ALTER TABLE device ADD CONSTRAINT fk_device_user FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device DROP CONSTRAINT fk_device_user');
        $this->addSql('DROP TABLE device');
    }
}
