<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/** Undoing a device removal while the install has not been told (D81). */
final class Version20261002110000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Add device.removal_noticed_at: until then, a removal can be undone';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device ADD removal_noticed_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device DROP removal_noticed_at');
    }
}
