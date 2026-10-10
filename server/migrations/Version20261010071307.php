<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/**
 * Auto-generated Migration: Please modify to your needs!
 */
final class Version20261010071307 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The key vouched for when a device was assigned to a person (7.5.2b-1, D96).';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_devices ADD vouched_key TEXT DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_devices DROP vouched_key');
    }
}
