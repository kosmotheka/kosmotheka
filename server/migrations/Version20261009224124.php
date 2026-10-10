<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261009224124 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The cloud\'s power in the protocol (7.5.1, D76, D95): the library names its cloud device and a member says it has an account.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_libraries ADD cloud_device_id TEXT DEFAULT NULL');
        $this->addSql('ALTER TABLE catalog_members ADD account_id TEXT DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE catalog_libraries DROP cloud_device_id');
        $this->addSql('ALTER TABLE catalog_members DROP account_id');
    }
}
