<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261009231728 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The cloud device of each cloud library (7.5.2a, D95): its id, its public key and its private key, encrypted.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_library ADD cloud_device_id UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE cloud_library ADD cloud_public_key VARCHAR(43) DEFAULT NULL');
        $this->addSql('ALTER TABLE cloud_library ADD cloud_secret_key TEXT DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_library DROP cloud_device_id');
        $this->addSql('ALTER TABLE cloud_library DROP cloud_public_key');
        $this->addSql('ALTER TABLE cloud_library DROP cloud_secret_key');
    }
}
