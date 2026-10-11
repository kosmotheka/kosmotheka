<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261010233029 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The activity log (7.7, D66): an append-only table with no foreign keys, so a line outlives what it records.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE activity_log (id UUID NOT NULL, kind VARCHAR(32) NOT NULL, occurred_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, channel VARCHAR(8) NOT NULL, account_id UUID DEFAULT NULL, account_email_fingerprint VARCHAR(64) DEFAULT NULL, device_id VARCHAR(64) DEFAULT NULL, library_id UUID DEFAULT NULL, library_name VARCHAR(200) DEFAULT NULL, subject_id VARCHAR(64) DEFAULT NULL, operation_id UUID DEFAULT NULL, detail VARCHAR(64) DEFAULT NULL, ip VARCHAR(45) DEFAULT NULL, user_agent VARCHAR(255) DEFAULT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_activity_log_occurred_at ON activity_log (occurred_at)');
        $this->addSql('CREATE INDEX idx_activity_log_account ON activity_log (account_id)');
        $this->addSql('CREATE INDEX idx_activity_log_library ON activity_log (library_id)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP TABLE activity_log');
    }
}
