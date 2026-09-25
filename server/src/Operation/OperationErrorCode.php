<?php

declare(strict_types=1);

namespace App\Operation;

/** The error codes of spec/README.md. */
enum OperationErrorCode: string
{
    case InvalidId = 'invalid_id';
    case InvalidIntent = 'invalid_intent';
    case NoChanges = 'no_changes';
    case UnknownField = 'unknown_field';
    case MissingField = 'missing_field';
    case NullRequired = 'null_required';
    case WrongType = 'wrong_type';
    case EmptyUpdate = 'empty_update';
    case FieldsOnDelete = 'fields_on_delete';
}
