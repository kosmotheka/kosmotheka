<?php

declare(strict_types=1);

namespace App\Sync\Permission;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\Role;
use App\Sync\LibraryId;
use App\Sync\NoLibraryIdException;

/**
 * Who may do what in a library (D55, D73): port of crates/domain/src/permission.rs. Pure rules, no database.
 */
final class Permissions
{
    private const int DAY_MILLIS = 24 * 60 * 60 * 1000;
    /** How long something stays in the trash before an editor may delete it for good (D105, rule 16). */
    public const int TRASH_DAYS = 30;
    /** How long before the cloud does it, as a safety net when no device has (D105, rule 16). */
    public const int CLOUD_TRASH_DAYS = 60;

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
        $author = $operation->author();
        // A fenced author is judged as nobody's: its date cannot be trusted, or its access was withdrawn (D71, D84).
        $fenced = $view->fenced($operation);
        // The operation that founds the library (D97): the only one allowed while nobody has a role. Whoever holds
        // the key the id derives from founds it, whatever date the operation carries.
        if (!$fenced && !$view->founded() && self::isFounding($operation)) {
            return true;
        }
        // A person leaving the library themself (D102, rule 15): for any role, so it is judged before them.
        if (!$fenced && self::isExit($operation, $author, $view)) {
            return true;
        }
        $role = $fenced ? null : $view->roleOfDevice($author);
        // The cloud's power (D76, D95) is the named device's, unless a fence makes it nobody's.
        // Asked only when a change could use it: its fields, or deleting from the trash (D105).
        $cloud = !$fenced
            && array_any($operation->changes, static fn (ChangePayload $change): bool => self::touchesCloudField($change) || ChangeAction::Delete === $change->action)
            && null !== ($named = $view->cloudDevice())
            && $named === $author;
        $at = (int) substr($operation->hlc, 0, 16);

        return array_all($operation->changes, static fn (ChangePayload $change): bool => self::changeAllowed($change, $author, $role, $cloud, $at, $view));
    }

    /**
     * A person leaving the library themself (D102, rule 15). The operation is signed by a device of that person
     * (`memberOfDevice`: its vouched key matches and their access is not withdrawn already) and holds exactly: their own
     * `member` row updated with `access_withdrawn: true` and nothing else; for devices that are hers now (their
     * `member_id`, as it stands), the take-away of the person and the vouched key, the same `device` update that
     * expelling writes (rule 14), the signing device also fencing itself at its previous operation (`trusted_through`)
     * and the others not fenced; and a `revocation` of such a device, with its name. Anything else, a device of another
     * person included, makes the whole operation a request.
     */
    private static function isExit(OperationPayload $operation, string $author, LibraryView $view): bool
    {
        $person = $view->memberOfDevice($author);
        if (null === $person) {
            return false;
        }
        $isHers = static fn (string $device): bool => null !== ($member = $view->rawMemberOfDevice($device)) && 0 === strcasecmp($member, $person);
        $exactly = static fn (ChangePayload $change, array $keys): bool => \count($change->fields) === \count($keys)
            && array_all($keys, static fn (mixed $key): bool => \is_string($key) && \array_key_exists($key, $change->fields));
        $marked = false;
        foreach ($operation->changes as $change) {
            if (CatalogEntity::Member === $change->entity && ChangeAction::Update === $change->action) {
                if ($marked || 0 !== strcasecmp($change->id, $person) || !$exactly($change, ['access_withdrawn']) || true !== $change->fields['access_withdrawn']) {
                    return false;
                }
                $marked = true;
            } elseif (CatalogEntity::Device === $change->entity && ChangeAction::Update === $change->action) {
                $own = $change->id === $author;
                if (!$exactly($change, $own ? ['member_id', 'vouched_key', 'trusted_through'] : ['member_id', 'vouched_key'])
                    || null !== $change->fields['member_id'] || null !== $change->fields['vouched_key'] || !$isHers($change->id)) {
                    return false;
                }
                // Everything the device did before is an ancestor of its previous operation: nothing of it is disbelieved.
                if ($own && ($operation->previous ?? '') !== $change->fields['trusted_through']) {
                    return false;
                }
            } elseif (CatalogEntity::Revocation === $change->entity && ChangeAction::Create === $change->action) {
                $device = $change->fields['device_id'] ?? null;
                if (!$exactly($change, ['device_id', 'device_name']) || !\is_string($change->fields['device_name']) || !\is_string($device) || !$isHers($device)) {
                    return false;
                }
            } else {
                return false;
            }
        }

        return $marked;
    }

    /**
     * Whether `$operation` is the founding of its library (D97, rule 1): it creates the `library` row with a
     * `founding_nonce`, and the public key its author gives in its own `device` row of the same operation, with that
     * nonce, derives the id of the library. Whether the library is still unfounded is the caller's to ask.
     */
    private static function isFounding(OperationPayload $operation): bool
    {
        $author = $operation->author();
        $nonce = $key = null;
        foreach ($operation->changes as $change) {
            if (CatalogEntity::Library === $change->entity && ChangeAction::Create === $change->action && 0 === strcasecmp($change->id, $operation->library)) {
                $nonce ??= \is_string($change->fields['founding_nonce'] ?? null) ? $change->fields['founding_nonce'] : null;
            }
            if (CatalogEntity::Device === $change->entity && ChangeAction::Delete !== $change->action && $change->id === $author) {
                $key ??= \is_string($change->fields['public_key'] ?? null) ? $change->fields['public_key'] : null;
            }
        }

        if (null === $nonce || null === $key) {
            return false;
        }
        try {
            return LibraryId::derive($key, $nonce) === strtolower($operation->library);
        } catch (NoLibraryIdException) {
            // A key or a nonce that are not one: nothing derives from them, so this founds nothing.
            return false;
        }
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
    private static function cloudAllowed(ChangePayload $change, string $author, int $at, LibraryView $view): bool
    {
        // Its one power over the catalog: deleting for good what has been 60 days in the trash (D105).
        if (ChangeAction::Delete === $change->action) {
            return self::inTrashFor($change, $at, self::CLOUD_TRASH_DAYS, $view);
        }
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

    /** Whether the row, or what it hangs from, has been in the trash at least `$days` at the moment `$at` (D105). */
    private static function inTrashFor(ChangePayload $change, int $at, int $days, LibraryView $view): bool
    {
        $since = $view->trashedSince($change->entity, $change->id);

        return null !== $since && $at >= $since + $days * self::DAY_MILLIS;
    }

    /**
     * What a change does with the trash fields (D105): null when it does not touch them, false when it restores
     * (both null), and otherwise the person it names in `trashed_by` ('' when it names none).
     */
    private static function trashMove(ChangePayload $change): string|false|null
    {
        if (!$change->entity->isTrashable() || !\array_key_exists('trashed_at', $change->fields)) {
            return null;
        }
        if (null === $change->fields['trashed_at']) {
            return false;
        }
        $by = $change->fields['trashed_by'] ?? null;

        return \is_string($by) ? $by : '';
    }

    private static function changeAllowed(ChangePayload $change, string $author, ?Role $role, bool $cloud, int $at, LibraryView $view): bool
    {
        // `founding_nonce` is written once, by the founding (judged before this is asked): nobody writes it
        // afterwards, an administrator included (D97).
        if (CatalogEntity::Library === $change->entity && \array_key_exists('founding_nonce', $change->fields)) {
            return false;
        }
        // `account_id` is written by the cloud device alone: not by an administrator, not when the member is created.
        if (CatalogEntity::Member === $change->entity && \array_key_exists('account_id', $change->fields)) {
            return $cloud && self::cloudAllowed($change, $author, $at, $view);
        }
        if ($cloud && self::cloudAllowed($change, $author, $at, $view)) {
            return true;
        }
        // Who sent something to the trash is the signer's person, an administrator's included: the date and the name
        // in the trash are compared, never taken as said (D84, D105).
        $move = self::trashMove($change);
        if (ChangeAction::Update === $change->action && \is_string($move)
            && (null === ($me = $view->memberOfDevice($author)) || 0 !== strcasecmp($me, $move))) {
            return false;
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

        return match ($change->action) {
            // Sending to the trash is deleting as it was before the trash (D105, rule 16): her person's rows only.
            // Restoring is any editor's, whoever sent it.
            ChangeAction::Update => !\is_string($move) || $view->createdBySamePerson($change->entity, $change->id, $author),
            // A row created already in the trash (restoring a backup) names a person of the library. An
            // administrator's restore creates the people in the same operation, so it is not asked.
            ChangeAction::Create => !\is_string($move) || ('' !== $move && $view->memberExists($move)),
            // For good: anything that has been 30 days in the trash, or hangs from something that has, whoever created
            // it. The four things of the trash never go straight; the other rows, as before (rule 3).
            ChangeAction::Delete => self::inTrashFor($change, $at, self::TRASH_DAYS, $view)
                || (!$change->entity->isTrashable() && $view->createdBySamePerson($change->entity, $change->id, $author)),
        };
    }
}
