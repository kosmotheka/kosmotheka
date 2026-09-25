<?php

declare(strict_types=1);

namespace App\Operation;

/**
 * Structural checks shared by every client and the server (spec/README.md).
 * Whether referenced rows exist, or whether the author may make these changes,
 * is checked when applying.
 *
 * Errors are reported in the order the spec fixes: operation id, intent, empty
 * changes, then each change in order (id, action, fields in alphabetical order,
 * missing required fields). Only the first one is returned.
 */
final class OperationValidator
{
    private const string UUID = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i';
    private const string HLC = '/^[0-9]{16}-[0-9a-f]{8}-.+$/';
    private const string INTENT = '/^[a-z_]+(\.[a-z_]+)+$/';
    private const array OPERATION_KEYS = ['id', 'hlc', 'intent', 'changes'];
    private const array CHANGE_KEYS = ['entity', 'id', 'action', 'fields'];

    /**
     * @param array<mixed> $operation a decoded JSON operation
     *
     * @throws MalformedOperation
     */
    public function validate(array $operation): ?OperationError
    {
        $this->assertShape($operation);

        if (!$this->isUuid($operation['id'])) {
            return new OperationError(OperationErrorCode::InvalidId);
        }
        if (1 !== preg_match(self::INTENT, $operation['intent'])) {
            return new OperationError(OperationErrorCode::InvalidIntent);
        }
        if ([] === $operation['changes']) {
            return new OperationError(OperationErrorCode::NoChanges);
        }
        foreach ($operation['changes'] as $index => $change) {
            $error = $this->validateChange($change);
            if ($error instanceof OperationError) {
                return $error->inChange($index);
            }
        }

        return null;
    }

    /**
     * @param array{entity: string, id: string, action: string, fields?: array<string, mixed>} $change
     */
    private function validateChange(array $change): ?OperationError
    {
        if (!$this->isUuid($change['id'])) {
            return new OperationError(OperationErrorCode::InvalidId);
        }

        $fields = $change['fields'] ?? [];
        if ('delete' === $change['action'] && [] !== $fields) {
            return new OperationError(OperationErrorCode::FieldsOnDelete);
        }
        if ('update' === $change['action'] && [] === $fields) {
            return new OperationError(OperationErrorCode::EmptyUpdate);
        }

        $specs = Entity::from($change['entity'])->fields();
        ksort($fields, \SORT_STRING);
        foreach ($fields as $name => $value) {
            $spec = $specs[$name] ?? null;
            if (null === $spec) {
                return new OperationError(OperationErrorCode::UnknownField, field: $name);
            }
            if (null === $value) {
                if ($spec->required) {
                    return new OperationError(OperationErrorCode::NullRequired, field: $name);
                }
            } elseif (!$this->matchesType($spec, $value)) {
                return new OperationError(OperationErrorCode::WrongType, field: $name);
            }
        }

        if ('create' === $change['action']) {
            foreach ($specs as $name => $spec) {
                if ($spec->required && !\array_key_exists($name, $fields)) {
                    return new OperationError(OperationErrorCode::MissingField, field: $name);
                }
            }
        }

        return null;
    }

    private function matchesType(FieldSpec $spec, mixed $value): bool
    {
        return match ($spec->type) {
            FieldType::Text => \is_string($value),
            FieldType::Integer => \is_int($value),
            FieldType::Date => \is_string($value) && $this->isDate($value),
            FieldType::OneOf => \in_array($value, $spec->allowed, true),
            FieldType::Ref => $this->isUuid($value),
        };
    }

    private function isUuid(mixed $value): bool
    {
        return \is_string($value) && 1 === preg_match(self::UUID, $value);
    }

    private function isDate(string $value): bool
    {
        if (1 !== preg_match('/^(\d{4})-(\d{2})-(\d{2})$/', $value, $parts)) {
            return false;
        }

        return checkdate((int) $parts[2], (int) $parts[3], (int) $parts[1]);
    }

    /**
     * @param array<mixed> $operation
     *
     * @phpstan-assert array{id: mixed, hlc: string, intent: string, changes: list<array{entity: string, id: mixed, action: string, fields?: array<string, mixed>}>} $operation
     */
    private function assertShape(array $operation): void
    {
        $this->assertKeys($operation, self::OPERATION_KEYS, self::OPERATION_KEYS, 'operation');
        if (!\is_string($operation['hlc']) || 1 !== preg_match(self::HLC, $operation['hlc'])) {
            throw new MalformedOperation('hlc must look like 0001727222222222-00000000-<device>');
        }
        if (!\is_string($operation['intent'])) {
            throw new MalformedOperation('intent must be a string');
        }
        if (!\is_array($operation['changes']) || !array_is_list($operation['changes'])) {
            throw new MalformedOperation('changes must be a list');
        }
        foreach ($operation['changes'] as $index => $change) {
            if (!\is_array($change)) {
                throw new MalformedOperation(\sprintf('change %d must be an object', $index));
            }
            $this->assertKeys($change, ['entity', 'id', 'action'], self::CHANGE_KEYS, "change $index");
            if (!\is_string($change['entity']) || null === Entity::tryFrom($change['entity'])) {
                throw new MalformedOperation(\sprintf('change %d has an unknown entity', $index));
            }
            if (!\in_array($change['action'], ['create', 'update', 'delete'], true)) {
                throw new MalformedOperation(\sprintf('change %d has an unknown action', $index));
            }
            // An empty JSON object decodes to [] too, so only a list with
            // elements gives away that fields was not an object.
            if (\array_key_exists('fields', $change)
                && (!\is_array($change['fields']) || ([] !== $change['fields'] && array_is_list($change['fields'])))) {
                throw new MalformedOperation(\sprintf('change %d: fields must be an object', $index));
            }
        }
    }

    /**
     * @param array<mixed> $value
     * @param list<string> $required
     * @param list<string> $allowed
     */
    private function assertKeys(array $value, array $required, array $allowed, string $what): void
    {
        foreach ($required as $key) {
            if (!\array_key_exists($key, $value)) {
                throw new MalformedOperation(\sprintf('%s is missing "%s"', $what, $key));
            }
        }
        $unknown = array_diff(array_keys($value), $allowed);
        if ([] !== $unknown) {
            throw new MalformedOperation(\sprintf('%s has unknown keys: %s', $what, implode(', ', $unknown)));
        }
    }
}
