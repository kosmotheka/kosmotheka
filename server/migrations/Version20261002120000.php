<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/** Second factor: TOTP on the account and its backup codes (D78). */
final class Version20261002120000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Add TOTP columns to user and create the backup_code table';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE "user" ADD totp_secret VARCHAR(255) DEFAULT NULL');
        $this->addSql('ALTER TABLE "user" ADD totp_pending_secret VARCHAR(255) DEFAULT NULL');
        $this->addSql('ALTER TABLE "user" ADD totp_enabled_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL');
        $this->addSql('ALTER TABLE "user" ADD totp_last_timestep INT DEFAULT NULL');
        $this->addSql('CREATE TABLE backup_code (id UUID NOT NULL, code_hash VARCHAR(64) NOT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, used_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, user_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_backup_code_user ON backup_code (user_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_backup_code_user_hash ON backup_code (user_id, code_hash)');
        $this->addSql('ALTER TABLE backup_code ADD CONSTRAINT fk_backup_code_user FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE backup_code DROP CONSTRAINT fk_backup_code_user');
        $this->addSql('DROP TABLE backup_code');
        $this->addSql('ALTER TABLE "user" DROP totp_secret');
        $this->addSql('ALTER TABLE "user" DROP totp_pending_secret');
        $this->addSql('ALTER TABLE "user" DROP totp_enabled_at');
        $this->addSql('ALTER TABLE "user" DROP totp_last_timestep');
    }
}
