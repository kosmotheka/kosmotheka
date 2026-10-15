<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261015193814 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'D99: the owner of a cloud library no longer takes it with her account (RESTRICT), and the library remembers the key that signed its link';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_library DROP CONSTRAINT fk_cloud_library_owner');
        $this->addSql('ALTER TABLE cloud_library ADD link_signer_key VARCHAR(43) DEFAULT NULL');
        $this->addSql('ALTER TABLE cloud_library ADD CONSTRAINT fk_cloud_library_owner FOREIGN KEY (owner_id) REFERENCES "user" (id) ON DELETE RESTRICT NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE cloud_library DROP CONSTRAINT fk_cloud_library_owner');
        $this->addSql('ALTER TABLE cloud_library DROP link_signer_key');
        $this->addSql('ALTER TABLE cloud_library ADD CONSTRAINT fk_cloud_library_owner FOREIGN KEY (owner_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE INITIALLY IMMEDIATE');
    }
}
