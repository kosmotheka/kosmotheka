<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/** Why the server signed a device out, so the app can say so (D50). */
final class Version20261002130000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Add device.sign_out_reason: tells the app it was signed out because the password changed';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device ADD sign_out_reason VARCHAR(32) DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE device DROP sign_out_reason');
    }
}
