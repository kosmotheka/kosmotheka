<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/** A removed device, distinct from a signed-out one (D81). */
final class Version20261002100000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Add device.removed_at: a removed install can never sign in again';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device ADD removed_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device DROP removed_at');
    }
}
