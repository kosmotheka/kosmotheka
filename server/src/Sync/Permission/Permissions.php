<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\Role;
use App\Sync\LibraryId;

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
        // A fenced author is judged as nobody's: its date cannot be trusted, or its access was withdrawn (D71, D84).
        $fenced = $view->fenced($operation);
        // The operation that founds the library (D97): the only one allowed while nobody has a role. Whoever holds
        // the key the id derives from founds it, whatever date the operation carries.
        if (!$fenced && !$view->founded() && self::isFounding($operation)) {
            return true;
        }
        $role = $fenced ? null : $view->roleOfDevice($author);
        // The cloud's power (D76, D95) is the named device's, unless a fence makes it nobody's.
        // Asked only when a change could use it.
        $cloud = !$fenced
            && array_any($operation->changes, self::touchesCloudField(...))
            && null !== ($named = $view->cloudDevice())
            && $named === $author;

        return array_all($operation->changes, static fn (ChangePayload $change): bool => self::changeAllowed($change, $author, $role, $cloud, $view));
    }

    /**
     * Whether `$operation` is the founding of its library (D97, rule 1): it creates the `library` row with a
     * `founding_nonce`, and the public key its author gives in its own `device` row of the same operation, with that
     * nonce, derives the id of the library. Whether the library is still unfounded is the caller's to ask.
     */
    private static function isFounding(OperationPayload $operation): bool
    {
        $author = self::author($operation);
        $nonce = $key = null;
        foreach ($operation->changes as $change) {
            if (CatalogEntity::Library === $change->entity && ChangeAction::Create === $change->action && 0 === strcasecmp($change->id, $operation->library)) {
                $nonce ??= \is_string($change->fields['founding_nonce'] ?? null) ? $change->fields['founding_nonce'] : null;
            }
            if (CatalogEntity::Device === $change->entity && ChangeAction::Delete !== $change->action && $change->id === $author) {
                $key ??= \is_string($change->fields['public_key'] ?? null) ? $change->fields['public_key'] : null;
            }
        }

        return null !== $nonce && null !== $key && LibraryId::derive($key, $nonce) === strtolower($operation->library);
    }

    /** Whether the change writes `member.account_id` or `device.member_id` / `device.vouched_key`, the fields the cloud device has a say in (D76, D96). */
    private static function touchesCloudField(ChangePayload $change): bool
    {
        return match ($change->entity) {
            CatalogEntity::Member => \array_key_exists('account_id', $change->fields),
            CatalogEntity::Device => \array_key_exists('member_id', $change->fields) || \array_key_exists('vouched_key', $change->fields),
            default => false,
        };
    }

    /**
     * The two changes the cloud device may make besides what a device of nobody may do (D76, D96): the `account_id` of
     * a person who exists, which is all the change touches; and the link of another device that belongs to nobody to a
     * person with an account, which touches exactly `member_id` and `vouched_key`, both set. The cloud vouches for what
     * it links; whether that key is the registered one is rule 13's business.
     */
    private static function cloudAllowed(ChangePayload $change, string $author, LibraryView $view): bool
    {
        if (ChangeAction::Update !== $change->action) {
            return false;
        }

        return match ($change->entity) {
            CatalogEntity::Member => 1 === \count($change->fields) && \array_key_exists('account_id', $change->fields) && $view->memberExists($change->id),
            CatalogEntity::Device => 2 === \count($change->fields)
                && \is_string($change->fields['vouched_key'] ?? null) && '' !== $change->fields['vouched_key']
                && $change->id !== $author
                && !$view->deviceHasMemberId($change->id)
                && \is_string($change->fields['member_id'] ?? null) && '' !== $change->fields['member_id']
                && $view->memberHasAccount($change->fields['member_id'])
                && !$view->memberAccessWithdrawn($change->fields['member_id']),
            default => false,
        };
    }

    /** The device in the HLC: `<time>-<counter>-<device>`. */
    public static function author(OperationPayload $operation): string
    {
        return explode('-', $operation->hlc, 3)[2] ?? '';
    }

    private static function changeAllowed(ChangePayload $change, string $author, ?Role $role, bool $cloud, LibraryView $view): bool
    {
        // `founding_nonce` is written once, by the founding (judged before this is asked): nobody writes it
        // afterwards, an administrator included (D97).
        if (CatalogEntity::Library === $change->entity && \array_key_exists('founding_nonce', $change->fields)) {
            return false;
        }
        // `account_id` is written by the cloud device alone: not by an administrator, not when the member is created.
        if (CatalogEntity::Member === $change->entity && \array_key_exists('account_id', $change->fields)) {
            return $cloud && self::cloudAllowed($change, $author, $view);
        }
        if ($cloud && self::cloudAllowed($change, $author, $view)) {
            return true;
        }
        $someone = $role instanceof Role;
        $role ??= Role::Reader;
        if (Role::Admin === $role) {
            return true;
        }
        $touches = static fn (string $field): bool => \array_key_exists($field, $change->fields);

        // Every device registers and renames itself; saying whose it is (and vouching for its key, D96), or
        // touching another device, or its fence, is for administrators.
        if (CatalogEntity::Device === $change->entity && $change->id === $author) {
            return ChangeAction::Delete !== $change->action && !$touches('member_id') && !$touches('vouched_key') && !$touches('trusted_through');
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
                // Deleting a person is for administrators, even one the editor created herself (SEG-034).
                if (ChangeAction::Delete === $change->action) {
                    return false;
                }
                if ($touches('role') || $touches('can_share') || $touches('access_withdrawn')) {
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
