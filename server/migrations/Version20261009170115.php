<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261009170115 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Who uploaded each operation of the log (7.4.6), apart from who signed it.';
    }

    public function up(Schema $schema): void
    {
        // Null in the rows that were already there: who signed an operation and who uploaded it are not the same person.
        $this->addSql('ALTER TABLE cloud_operation ADD pushed_by_device_id VARCHAR(36) DEFAULT NULL');
        $this->addSql('ALTER TABLE cloud_operation ADD pushed_by_user_id UUID DEFAULT NULL');
        $this->addSql('ALTER TABLE cloud_operation ADD CONSTRAINT fk_cloud_operation_pushed_by_user FOREIGN KEY (pushed_by_user_id) REFERENCES "user" (id) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('CREATE INDEX idx_cloud_operation_pushed_by_user ON cloud_operation (pushed_by_user_id)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_operation DROP CONSTRAINT fk_cloud_operation_pushed_by_user');
        $this->addSql('DROP INDEX idx_cloud_operation_pushed_by_user');
        $this->addSql('ALTER TABLE cloud_operation DROP pushed_by_device_id');
        $this->addSql('ALTER TABLE cloud_operation DROP pushed_by_user_id');
    }
}
