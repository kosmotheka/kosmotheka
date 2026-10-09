<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\Role;

/**
 * Who may do what in a library (D55, D73): port of crates/domain/src/permission.rs. Pure rules, no database.
 */
final class Permissions
{
    /**
     * Whether the author of `$operation` may make all of its changes. All or nothing:
     * one change not allowed makes the whole operation a request.
     */
    public static function isAllowed(OperationPayload $operation, LibraryView $view): bool
    {
        // Every operation is signed (D56): without a signature nobody can tell who wrote it.
        if (null === $operation->signature) {
            return false;
        }
        $author = self::author($operation);
        // A fenced author is judged as nobody's, with or without roles: its date cannot
        // be trusted, or its access was withdrawn (D71, D84).
        $fenced = $view->fenced($operation);
        if (!$view->hasRoles() && !$fenced) {
            return true;
        }
        $role = $fenced ? null : $view->roleOfDevice($author);

        return array_all($operation->changes, static fn (ChangePayload $change): bool => self::changeAllowed($change, $author, $role, $view));
    }

    /** The device in the HLC: `<time>-<counter>-<device>`. */
    public static function author(OperationPayload $operation): string
    {
        return explode('-', $operation->hlc, 3)[2] ?? '';
    }

    private static function changeAllowed(ChangePayload $change, string $author, ?Role $role, LibraryView $view): bool
    {
        $someone = $role instanceof Role;
        $role ??= Role::Reader;
        if (Role::Admin === $role) {
            return true;
        }
        $touches = static fn (string $field): bool => \array_key_exists($field, $change->fields);

        // Every device registers and renames itself; saying whose it is, or
        // touching another device, or its fence, is for administrators.
        if (CatalogEntity::Device === $change->entity && $change->id === $author) {
            return ChangeAction::Delete !== $change->action && !$touches('member_id') && !$touches('trusted_through');
        }

        switch ($change->entity) {
            // Withdrawing is the one resolution that is not an administrator's.
            case CatalogEntity::ChangeRequest:
                return ChangeAction::Create === $change->action
                    && 1 === \count($change->fields)
                    && 'withdrawn' === ($change->fields['status'] ?? null)
                    && $view->requestedBySamePerson($change->id, $author);
            case CatalogEntity::Device:
            case CatalogEntity::Library:
            case CatalogEntity::Revocation:
                return false;
                // Nobody gives more than they have.
            case CatalogEntity::Member:
                if ($touches('role') || $touches('can_share')) {
                    return false;
                }
                break;
                // Each person keeps their own marks, whatever their role (D68).
            case CatalogEntity::Mark:
                $me = $someone ? $view->memberOfDevice($author) : null;
                if (null === $me) {
                    return false;
                }
                $owner = ChangeAction::Create === $change->action
                    ? (\is_string($change->fields['member_id'] ?? null) ? $change->fields['member_id'] : null)
                    : $view->markMember($change->id);

                return null !== $owner && 0 === strcasecmp($owner, $me);
            default:
                break;
        }

        if (Role::Reader === $role) {
            return false;
        }

        return ChangeAction::Delete !== $change->action || $view->createdBySamePerson($change->entity, $change->id, $author);
    }
}
