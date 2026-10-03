<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261003143057 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'A device is its id together with its public key (D84, SEG-016).';
    }

    public function up(Schema $schema): void
    {
        // Existing rows keep their id as the app's device id and as primary key.
        $this->addSql('ALTER TABLE device ADD device_id VARCHAR(36)');
        $this->addSql('UPDATE device SET device_id = id');
        $this->addSql('ALTER TABLE device ALTER device_id SET NOT NULL');
        $this->addSql('ALTER TABLE device ALTER id TYPE UUID USING id::uuid');
        $this->addSql('CREATE UNIQUE INDEX uniq_device_id_key ON device (device_id, public_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_device_user_device_id ON device (user_id, device_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_device_token_hash ON device (token_hash)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP INDEX uniq_device_id_key');
        $this->addSql('DROP INDEX uniq_device_user_device_id');
        $this->addSql('DROP INDEX uniq_device_token_hash');
        $this->addSql('ALTER TABLE device DROP device_id');
        $this->addSql('ALTER TABLE device ALTER id TYPE VARCHAR(36)');
    }
}
