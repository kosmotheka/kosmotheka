<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261010082211 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'The public id of an account, the one-account-per-person rule of a library and the index the account claim reads (7.5.2b-2, D76, D96).';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE INDEX idx_catalog_devices_vouched_key ON catalog_devices (vouched_key)');
        $this->addSql('CREATE UNIQUE INDEX uniq_membership_cloud_library_member ON membership (cloud_library_id, member_id)');
        // Accounts that already exist get a random id each (v4), then the column becomes required.
        $this->addSql('ALTER TABLE "user" ADD account_id UUID DEFAULT NULL');
        $this->addSql('UPDATE "user" SET account_id = gen_random_uuid()');
        $this->addSql('ALTER TABLE "user" ALTER account_id SET NOT NULL');
        $this->addSql('CREATE UNIQUE INDEX uniq_user_account_id ON "user" (account_id)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP INDEX idx_catalog_devices_vouched_key');
        $this->addSql('DROP INDEX uniq_membership_cloud_library_member');
        $this->addSql('DROP INDEX uniq_user_account_id');
        $this->addSql('ALTER TABLE "user" DROP account_id');
    }
}
