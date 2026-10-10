<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261010101344 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'A person whose access was withdrawn (7.5.4a, D98): they stay in the catalog.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_members ADD access_withdrawn BOOLEAN DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_members DROP access_withdrawn');
    }
}
