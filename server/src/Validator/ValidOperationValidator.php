<?php

declare(strict_types=1);

namespace App\Validator;

use App\Dto\ChangePayload;
use App\Dto\FieldSpec;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\ChangeAction;
use App\Enum\FieldType;
use App\Enum\OperationErrorCode;
use Symfony\Component\Validator\Constraint;
use Symfony\Component\Validator\ConstraintValidator;
use Symfony\Component\Validator\Exception\UnexpectedTypeException;
use Symfony\Component\Validator\Exception\UnexpectedValueException;

/**
 * Port of Operation::validate in crates/domain/src/operation.rs. Both run the
 * cases in spec/conformance/operation-validation.json.
 */
final class ValidOperationValidator extends ConstraintValidator
{
    private const string UUID = '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i';
    private const string INTENT = '/^[a-z_]+(\.[a-z_]+)+$/';

    public function validate(mixed $value, Constraint $constraint): void
    {
        if (!$constraint instanceof ValidOperation) {
            throw new UnexpectedTypeException($constraint, ValidOperation::class);
        }
        if (!$value instanceof OperationPayload) {
            throw new UnexpectedValueException($value, OperationPayload::class);
        }

        if (!$this->isUuid($value->id)) {
            $this->report(OperationErrorCode::InvalidId, 'id');

            return;
        }
        if (!$this->isUuid($value->library)) {
            $this->report(OperationErrorCode::InvalidLibrary, 'library');

            return;
        }
        if (1 !== preg_match(self::INTENT, $value->intent)) {
            $this->report(OperationErrorCode::InvalidIntent, 'intent');

            return;
        }
        if ([] === $value->changes) {
            $this->report(OperationErrorCode::NoChanges, 'changes');

            return;
        }
        foreach ($value->changes as $index => $change) {
            if ($this->validateChange($change, $value->library, $this->author($value->hlc), "changes[$index]")) {
                return;
            }
        }
    }

    /**
     * @param string $library the operation's own, the only library a change may touch
     * @param string $device  the operation's author, the only device that may register itself
     *
     * @return bool whether a violation was reported
     */
    private function validateChange(ChangePayload $change, string $library, string $device, string $path): bool
    {
        if (!$this->isUuid($change->id)) {
            return $this->report(OperationErrorCode::InvalidId, "$path.id");
        }
        if (CatalogEntity::Library === $change->entity && $change->id !== $library) {
            return $this->report(OperationErrorCode::OtherLibrary, "$path.id");
        }
        if (CatalogEntity::Device === $change->entity && ChangeAction::Create === $change->action && $change->id !== $device) {
            return $this->report(OperationErrorCode::OtherDevice, "$path.id");
        }
        if (ChangeAction::Delete === $change->action && [] !== $change->fields) {
            return $this->report(OperationErrorCode::FieldsOnDelete, "$path.fields");
        }
        if (ChangeAction::Update === $change->action && [] === $change->fields) {
            return $this->report(OperationErrorCode::EmptyUpdate, "$path.fields");
        }

        $specs = $change->entity->fields();
        $fields = $change->fields;
        ksort($fields, \SORT_STRING);
        foreach ($fields as $name => $value) {
            $spec = $specs[$name] ?? null;
            if (null === $spec) {
                return $this->report(OperationErrorCode::UnknownField, "$path.fields[$name]");
            }
            if (null === $value && $spec->required) {
                return $this->report(OperationErrorCode::NullRequired, "$path.fields[$name]");
            }
            if (null !== $value && !$this->matchesType($spec, $value)) {
                return $this->report(OperationErrorCode::WrongType, "$path.fields[$name]");
            }
        }

        if (ChangeAction::Create === $change->action) {
            foreach ($specs as $name => $spec) {
                if ($spec->required && !\array_key_exists($name, $fields)) {
                    return $this->report(OperationErrorCode::MissingField, "$path.fields[$name]");
                }
            }
        }

        return false;
    }

    private function matchesType(FieldSpec $spec, mixed $value): bool
    {
        return match ($spec->type) {
            FieldType::Text => \is_string($value),
            FieldType::Integer => \is_int($value),
            FieldType::Date => \is_string($value) && $this->isDate($value),
            FieldType::OneOf => \in_array($value, $spec->allowed, true),
            FieldType::Ref => \is_string($value) && $this->isUuid($value),
        };
    }

    /**
     * The device in a hybrid logical clock timestamp: what follows the time and the counter.
     */
    private function author(string $hlc): string
    {
        return explode('-', $hlc, 3)[2] ?? '';
    }

    private function isUuid(string $value): bool
    {
        return 1 === preg_match(self::UUID, $value);
    }

    private function isDate(string $value): bool
    {
        if (1 !== preg_match('/^(\d{4})-(\d{2})-(\d{2})$/', $value, $parts)) {
            return false;
        }

        return checkdate((int) $parts[2], (int) $parts[3], (int) $parts[1]);
    }

    private function report(OperationErrorCode $code, string $path): true
    {
        $this->context->buildViolation('Invalid operation: {{ code }}.')
            ->setParameter('{{ code }}', $code->value)
            ->setCode($code->value)
            ->atPath($path)
            ->addViolation();

        return true;
    }
}
