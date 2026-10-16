<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use App\Security\CloudKeyVault;
use App\Security\CloudKeyVaultException;
use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261016120000 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Cloud keys: the private keys sealed before SEG-074 are sealed again, tied to their library';
    }

    public function up(Schema $schema): void
    {
        $rows = $this->connection->fetchAllAssociative("SELECT id, cloud_secret_key FROM cloud_library WHERE cloud_secret_key IS NOT NULL AND cloud_secret_key NOT LIKE 'v1.%'");
        if ([] === $rows) {
            return;
        }
        // Read only when there is something to seal again: the secret the server opens them with.
        $secret = $_SERVER['CLOUD_KEY_SECRET'] ?? $_ENV['CLOUD_KEY_SECRET'] ?? '';
        $this->abortIf(!\is_string($secret) || '' === $secret, 'CLOUD_KEY_SECRET is needed to seal the cloud keys again; nothing was changed.');
        $vault = new CloudKeyVault($secret);

        // All or nothing: every key is opened and sealed before anything is queued, so a key that does not open (another
        // secret, a damaged value) stops the migration with the database as it was.
        $sealed = [];
        foreach ($rows as $row) {
            $id = (string) $row['id'];
            try {
                $sealed[$id] = $vault->seal($id, $vault->open($id, (string) $row['cloud_secret_key']));
            } catch (CloudKeyVaultException $e) {
                $this->abortIf(true, \sprintf('The cloud key of library %s does not open with CLOUD_KEY_SECRET (%s); nothing was changed.', $id, $e->getMessage()));
            }
        }
        foreach ($sealed as $id => $value) {
            $this->addSql('UPDATE cloud_library SET cloud_secret_key = :sealed WHERE id = :id', ['sealed' => $value, 'id' => $id]);
        }
    }

    public function down(Schema $schema): void
    {
        $this->throwIrreversibleMigrationException('The keys are not sealed again in the old format: restore the copy of the database taken before.');
    }
}
