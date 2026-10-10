<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261010104714 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'When the server saw that the person of an account had the access withdrawn (7.5.4b, D98).';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE membership ADD withdrawn_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE membership DROP withdrawn_at');
    }
}
