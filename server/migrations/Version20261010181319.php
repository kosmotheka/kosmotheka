<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261010181319 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Invitations to a cloud library (7.6.1, D43): one row per invitation, with the fingerprints of its token and email.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE invitation (id UUID NOT NULL, created_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, accepted_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, revoked_at TIMESTAMP(0) WITHOUT TIME ZONE DEFAULT NULL, member_id UUID NOT NULL, kind VARCHAR(8) NOT NULL, token_hash VARCHAR(64) NOT NULL, expires_at TIMESTAMP(0) WITHOUT TIME ZONE NOT NULL, email_hash VARCHAR(64) DEFAULT NULL, accepted_by_id UUID DEFAULT NULL, cloud_library_id UUID NOT NULL, created_by_id UUID NOT NULL, PRIMARY KEY (id))');
        $this->addSql('CREATE INDEX idx_invitation_cloud_library ON invitation (cloud_library_id)');
        $this->addSql('CREATE INDEX idx_invitation_created_by ON invitation (created_by_id)');
        $this->addSql('CREATE INDEX idx_invitation_accepted_by ON invitation (accepted_by_id)');
        $this->addSql('CREATE UNIQUE INDEX uniq_invitation_token_hash ON invitation (token_hash)');
        $this->addSql('ALTER TABLE invitation ADD CONSTRAINT fk_invitation_accepted_by FOREIGN KEY (accepted_by_id) REFERENCES "user" (id) ON DELETE SET NULL NOT DEFERRABLE');
        $this->addSql('ALTER TABLE invitation ADD CONSTRAINT fk_invitation_cloud_library FOREIGN KEY (cloud_library_id) REFERENCES cloud_library (id) ON DELETE CASCADE NOT DEFERRABLE');
        $this->addSql('ALTER TABLE invitation ADD CONSTRAINT fk_invitation_created_by FOREIGN KEY (created_by_id) REFERENCES "user" (id) ON DELETE CASCADE NOT DEFERRABLE');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('ALTER TABLE invitation DROP CONSTRAINT fk_invitation_accepted_by');
        $this->addSql('ALTER TABLE invitation DROP CONSTRAINT fk_invitation_cloud_library');
        $this->addSql('ALTER TABLE invitation DROP CONSTRAINT fk_invitation_created_by');
        $this->addSql('DROP TABLE invitation');
    }
}
