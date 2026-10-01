<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/** Accounts and their email confirmation codes (D49, D50). */
final class Version20261001231337 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Create the user and email_code tables for server accounts (milestone 7.1)';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE email_code (id UUID NOT NULL, purpose VARCHAR(255) NOT NULL, code_hash VARCHAR(64) NOT NULL, expires_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, attempts INT NOT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, user_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_email_code_user ON email_code (user_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_email_code_user_purpose ON email_code (user_id, purpose)');
        $this->addSql('CREATE TABLE "user" (id UUID NOT NULL, email VARCHAR(180) NOT NULL, password VARCHAR(255) DEFAULT NULL, name VARCHAR(100) NOT NULL, locale VARCHAR(5) NOT NULL, roles JSON NOT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, confirmed_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, blocked_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE UNIQUE INDEX uniq_user_email ON "user" (email)');
        $this->addSql('ALTER TABLE email_code ADD CONSTRAINT fk_email_code_user FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE email_code DROP CONSTRAINT fk_email_code_user');
        $this->addSql('DROP TABLE email_code');
        $this->addSql('DROP TABLE "user"');
    }
}
