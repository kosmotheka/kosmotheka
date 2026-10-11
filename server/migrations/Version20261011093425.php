<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/**
 * Auto-generated Migration: Please modify to your needs!
 */
final class Version20261011093425 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Activity log: who uploaded a line that comes from an operation, apart from its author (SEG-041).';
    }

    public function up(Schema $schema): void
    {
        // this up() migration is auto-generated, please modify it to your needs
        $this->addSql('ALTER TABLE activity_log ADD uploaded_by_account_id UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE activity_log ADD uploaded_by_device_id VARCHAR(64) DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        // this down() migration is auto-generated, please modify it to your needs
        $this->addSql('ALTER TABLE activity_log DROP uploaded_by_account_id');
        $this->addSql('ALTER TABLE activity_log DROP uploaded_by_device_id');
    }
}
