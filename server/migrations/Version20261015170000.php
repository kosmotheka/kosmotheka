<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use App\Security\TwoFactorSecretBox;
use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261015170000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Second factor: the TOTP secrets stored in clear are sealed for their account (SEG-063)';
    }

    public function up(Schema $schema): void
    {
        $rows = $this->connection->fetchAllAssociative(
            "SELECT id, totp_secret, totp_pending_secret FROM \"user\" WHERE (totp_secret IS NOT NULL AND totp_secret NOT LIKE 'v1.%') OR (totp_pending_secret IS NOT NULL AND totp_pending_secret NOT LIKE 'v1.%')",
        );
        if ([] === $rows) {
            return;
        }
        // Read only when there is something to seal: the key is the one the server will open them with.
        $appSecret = $_SERVER['APP_SECRET'] ?? $_ENV['APP_SECRET'] ?? '';
        $this->abortIf(!\is_string($appSecret) || '' === $appSecret, 'APP_SECRET is needed to seal the second-factor secrets.');
        $key = TwoFactorSecretBox::keyFrom($appSecret);

        foreach ($rows as $row) {
            $id = (string) $row['id'];
            foreach (['totp_secret', 'totp_pending_secret'] as $column) {
                $value = $row[$column];
                if (\is_string($value) && !str_starts_with($value, 'v1.')) {
                    $this->addSql(\sprintf('UPDATE "user" SET %s = :sealed WHERE id = :id', $column), ['sealed' => TwoFactorSecretBox::sealWith($key, $id, $value), 'id' => $id]);
                }
            }
        }
    }

    public function down(Schema $schema): void
    {
        $this->throwIrreversibleMigrationException('The secrets are not written back in clear.');
    }
}
