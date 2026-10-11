<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/**
 * Auto-generated Migration: Please modify to your needs!
 */
final class Version20261011121213 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Continuar con Google (D77): the attempts, and the Google identifier of an account.';
    }

    public function up(Schema $schema): void
    {
        // this up() migration is auto-generated, please modify it to your needs
        $this->addSql('CREATE TABLE google_attempt (phase VARCHAR(16) NOT NULL, state_hash VARCHAR(64) DEFAULT NULL, binder_hash VARCHAR(64) DEFAULT NULL, pkce_verifier VARCHAR(200) DEFAULT NULL, code_hash VARCHAR(64) DEFAULT NULL, code_expires_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, failures INT NOT NULL, id VARCHAR(64) NOT NULL, verifier_hash VARCHAR(64) NOT NULL, locale VARCHAR(5) NOT NULL, expires_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, user_id UUID DEFAULT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_google_attempt_expires ON google_attempt (expires_at)');
        $this->addSql('CREATE UNIQUE INDEX uniq_google_attempt_state ON google_attempt (state_hash)');
        $this->addSql('CREATE INDEX IDX_E6A83FE0A76ED395 ON google_attempt (user_id)');
        $this->addSql('ALTER TABLE google_attempt ADD CONSTRAINT fk_google_attempt_user FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE "user" ADD google_sub VARCHAR(255) DEFAULT NULL');
        $this->addSql('CREATE UNIQUE INDEX uniq_user_google_sub ON "user" (google_sub)');
    }

    public function down(Schema $schema): void
    {
        // this down() migration is auto-generated, please modify it to your needs
        $this->addSql('ALTER TABLE google_attempt DROP CONSTRAINT fk_google_attempt_user');
        $this->addSql('DROP TABLE google_attempt');
        $this->addSql('DROP INDEX uniq_user_google_sub');
        $this->addSql('ALTER TABLE "user" DROP google_sub');
    }
}
