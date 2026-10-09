<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261009170909 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Which operation of the log links a cloud library (7.4.6, D93).';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_library ADD link_operation_id UUID DEFAULT NULL');
        // The libraries already linked: their earliest link operation in the log, found as CloudLink::isLink does.
        $this->addSql("UPDATE cloud_library l SET link_operation_id = (
            SELECT o.operation_id FROM cloud_operation o
            WHERE o.cloud_library_id = l.id AND EXISTS (
                SELECT 1 FROM jsonb_array_elements(o.body -> 'changes') c
                WHERE c ->> 'entity' = 'library' AND c ->> 'action' = 'update'
                    AND lower(c -> 'fields' ->> 'cloud_library_id') = l.id::text AND lower(c ->> 'id') = lower(o.body ->> 'library'))
            ORDER BY o.hlc LIMIT 1) WHERE l.local_library_id IS NOT NULL");
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_library DROP link_operation_id');
    }
}
