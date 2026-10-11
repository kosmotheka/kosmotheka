<?php

declare(strict_types=1);

namespace App\Sync;

use App\Dto\OperationPayload;
use App\Enum\ActivityKind;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\Role;
use Symfony\Component\Uid\Uuid;

/**
 * What an operation that has been **applied** says for the activity log (D66, 7.7). It only reads the changes:
 * who did it, where and when come from the server (`ActivityLog`), never from here. An operation the judge left as
 * a request, or the server refused, never reaches this class.
 *
 * Only updates (and a revocation, which is a create): a library's first upload carries its whole history, whose
 * creations are not «something changed». The cloud's own operations (`$byCloud`) are the account link and the
 * device link of D76; the same change written by a device is another thing (a device assigned to a person).
 */
final class OperationActivity
{
    /**
     * @return list<array{kind: ActivityKind, subject: ?string, detail: ?string}>
     */
    public static function of(OperationPayload $operation, bool $byCloud): array
    {
        $events = [];
        foreach ($operation->changes as $change) {
            $fields = $change->fields;
            if (CatalogEntity::Revocation === $change->entity && ChangeAction::Create === $change->action) {
                $events[] = self::event(ActivityKind::DeviceAccessRevoked, self::uuid($fields['device_id'] ?? null));
                continue;
            }
            if (CatalogEntity::Library === $change->entity && ChangeAction::Delete === $change->action) {
                $events[] = self::event(ActivityKind::LibraryDeleted);
                continue;
            }
            if (ChangeAction::Update !== $change->action) {
                continue;
            }
            if (CatalogEntity::Library === $change->entity) {
                if (\is_string($fields['cloud_library_id'] ?? null)) {
                    $events[] = self::event(ActivityKind::LibraryUploaded);
                }
                if (\array_key_exists('cloud_device_id', $fields)) {
                    $device = self::uuid($fields['cloud_device_id']);
                    $events[] = self::event(null === $device ? ActivityKind::CloudPowerOff : ActivityKind::CloudPowerOn, $device);
                }
            } elseif (CatalogEntity::Member === $change->entity) {
                $role = \is_string($fields['role'] ?? null) ? Role::tryFrom($fields['role']) : null;
                if ($role instanceof Role) {
                    $events[] = self::event(ActivityKind::RoleChanged, self::uuid($change->id), $role->value);
                }
                if (\is_bool($fields['can_share'] ?? null)) {
                    $events[] = self::event(ActivityKind::CanShareChanged, self::uuid($change->id), $fields['can_share'] ? 'true' : 'false');
                }
                if (\is_bool($fields['access_withdrawn'] ?? null)) {
                    $events[] = self::event($fields['access_withdrawn'] ? ActivityKind::AccessWithdrawn : ActivityKind::AccessRestored, self::uuid($change->id));
                }
                if ($byCloud && \is_string($fields['account_id'] ?? null)) {
                    $events[] = self::event(ActivityKind::AccountLinked, self::uuid($change->id));
                }
            } elseif (CatalogEntity::Device === $change->entity && \is_string($fields['member_id'] ?? null)) {
                $events[] = self::event($byCloud ? ActivityKind::DeviceLinked : ActivityKind::DeviceAssigned, self::uuid($change->id), self::uuid($fields['member_id']));
            }
        }

        return $events;
    }

    /** @return array{kind: ActivityKind, subject: ?string, detail: ?string} */
    private static function event(ActivityKind $kind, ?string $subject = null, ?string $detail = null): array
    {
        return ['kind' => $kind, 'subject' => $subject, 'detail' => $detail];
    }

    /** An id that really is one, lower-cased, or null: the log never keeps free text from a client. */
    private static function uuid(mixed $value): ?string
    {
        return \is_string($value) && Uuid::isValid($value) ? strtolower($value) : null;
    }
}
