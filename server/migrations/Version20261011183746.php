<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261011183746 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Metadata cache: the code holds the fingerprint of a title search too, so it is 64 long (7.8, TMDB)';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE metadata_cache ALTER code TYPE VARCHAR(64)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE metadata_cache ALTER code TYPE VARCHAR(13)');
    }
}
