<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/**
 * Two operations of one cloud library never share an HLC: it names its device, so only a
 * modified app could sign two with one time, and every client would fail to log the second.
 */
final class Version20261004124551 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'One HLC per operation in a cloud library';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE UNIQUE INDEX uniq_cloud_operation_hlc ON cloud_operation (cloud_library_id, hlc)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP INDEX uniq_cloud_operation_hlc');
    }
}
