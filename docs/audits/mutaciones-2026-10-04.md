# Pruebas de mutación, 04/10/2026

Herramienta: `cargo-mutants` 27.1.0, sobre el commit `b9718bf`, con el código de fusión, permisos, firmas y reloj: `app/src-tauri/src/sync.rs`, `app/src-tauri/src/permissions.rs` y `crates/domain/src/{hlc,library,operation,permission,signature}.rs`. Resultado: 319 mutantes, 238 detectados, 27 inviables, 1 con tiempo agotado y 53 supervivientes. De estos, 14 del dominio eran falsos «no detectados» en la primera pasada, porque solo corrieron los tests del crate del dominio; se repitieron con `--test-workspace true` y quedaron los 39 de la tabla (uno es el que agotó el tiempo). Cada decisión de abajo se comprobó a mano: se aplicó el mutante, se vio fallar el test nuevo y se revirtió.

| Mutante | Decisión | Test o motivo |
|---|---|---|
| `permissions.rs:137:13` `\|\|` por `&&` en `same_person` | Detectado | `permissions::tests::the_same_person_is_the_same_device_or_two_devices_of_one_member` (falla en `same_person("ghost", "ghost")`) |
| `permissions.rs:144:5` `judged_by_order` devuelve `Ok(true)` | Detectado | `sync::tests::a_late_operation_in_a_library_without_roles_does_not_rebuild_it` (la fila plantada sin operación desaparece al reconstruir) |
| `permissions.rs:277:58` `&&` por `\|\|` en `fences_of` | Equivalente | `fences_of` solo ve operaciones ya validadas: `trusted_through` solo existe en `Device` y un borrado no lleva campos (`FieldsOnDelete`), así que el filtro de `filter_map` deja fuera lo mismo |
| `permissions.rs:326:5` `note_fences` devuelve `Ok(true)` | Detectado | `sync::tests::an_operation_that_arrives_in_order_does_not_rebuild_the_library` |
| `permissions.rs:368:34` `==` por `!=` en `replay` | Equivalente | Solo rellena el conjunto de ediciones que se reindexan, y `rebuild` añade después todas las ediciones de la biblioteca; reindexar un id que no es una edición no hace nada |
| `sync.rs:83:20` `+=` por `-=` en `MergeReport::add` | Detectado | `sync::tests::a_merge_report_adds_up_field_by_field` |
| `sync.rs:84:23` `+=` por `-=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:83:20` `+=` por `*=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:84:23` `+=` por `*=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:85:30` `+=` por `-=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:86:23` `+=` por `-=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:85:30` `+=` por `*=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:86:23` `+=` por `*=` en `MergeReport::add` | Detectado | el mismo |
| `sync.rs:154:41` `!=` por `==` en `ensure_baseline` | Detectado | `sync::tests::a_library_row_alone_is_not_worth_a_baseline` |
| `sync.rs:264:26` `+=` por `*=` en `merge_in` (`known`) | Detectado | `sync::tests::the_report_counts_known_operations_and_skipped_changes` |
| `sync.rs:293:39` `<` por `<=` en `merge_in` | Equivalente | Solo cambia con una operación nueva cuyo HLC es igual al más reciente de la biblioteca, y eso no llega a fusionarse: `operations.hlc` es `UNIQUE` y `log_operation` falla antes. Ver «Hallazgo» más abajo |
| `sync.rs:304:44` `+=` por `*=` en `merge_in` (`skipped_changes`) | Detectado | `sync::tests::the_report_counts_known_operations_and_skipped_changes` |
| `sync.rs:315:43` `&&` por `\|\|` en `merge_in` | Detectado | `sync::tests::a_fence_set_by_a_request_is_not_noted` y `an_operation_that_arrives_in_order_does_not_rebuild_the_library` |
| `sync.rs:369:5` `newest_hlc` devuelve `Some("xyzzy")` | Detectado | `sync::tests::an_operation_that_arrives_in_order_does_not_rebuild_the_library` |
| `sync.rs:393:26` `==` por `!=` en `signature_holds` | Detectado | `sync::tests::a_registered_device_cannot_change_a_key_even_signing_with_its_own` (el test viejo pasaba por otra razón: el teléfono no estaba registrado) |
| `sync.rs:410:17` `&&` por `\|\|` en `signature_holds` | Detectado | `sync::tests::only_a_device_registering_itself_may_bring_its_own_key` (un `Update` de sí mismo no es un registro) |
| `sync.rs:409:17` `&&` por `\|\|` en `signature_holds` | Detectado | el mismo (una clave ofrecida para otro dispositivo) |
| `sync.rs:469:69` `<` por `<=` en `apply_change` | Detectado | `sync::tests::the_same_hlc_does_not_overwrite_a_field` |
| `sync.rs:481:21` borrar `fields` de `Change` en `apply_change` | Detectado | `sync::tests::a_change_keeps_the_versions_of_the_fields_it_lost` |
| `permissions.rs:208:25` `==` por `!=` en `rebuild` (tiempo agotado) | Detectado (cuelga) | Con `!=` el bucle de `rebuild` no termina: la suite de `sync::` pasó de 24 s a más de 120 s con decenas de tests «running for over 60 seconds». Un cuelgue cuenta como detectado |
| `hlc.rs:20:49` `*` por `+` | Detectado | `hlc::tests::the_allowed_drift_is_one_hour` |
| `hlc.rs:20:44` `*` por `+` | Detectado | el mismo |
| `hlc.rs:20:49` `*` por `/` | Detectado | el mismo |
| `hlc.rs:20:44` `*` por `/` | Detectado | el mismo |
| `hlc.rs:65:51` `&&` por `\|\|` en `receive` | Detectado | `hlc::tests::receive_takes_the_counter_of_whichever_clock_decides_the_millisecond` |
| `hlc.rs:65:31` `==` por `!=` en `receive` | Detectado | el mismo |
| `hlc.rs:65:61` `==` por `!=` en `receive` | Detectado | el mismo |
| `hlc.rs:118:53` `\|\|` por `&&` en `from_str` | Detectado | `hlc::tests::a_malformed_text_form_is_rejected_whichever_part_is_wrong` |
| `hlc.rs:118:31` `\|\|` por `&&` en `from_str` | Detectado | el mismo |
| `library.rs:15:29` `>` por `>=` en `validate_library_name` | Detectado | `library::tests::a_name_of_exactly_the_maximum_length_is_valid` |
| `operation.rs:256:9` `&&` por `\|\|` en `is_valid_intent` | Detectado | `operation::tests::an_intent_is_two_or_more_lowercase_words_joined_by_dots` |
| `operation.rs:257:30` `&&` por `\|\|` en `is_valid_intent` | Detectado | el mismo |
| `permission.rs:91:27` guarda `change.id == author` por `true` | Detectado | `permission::tests::a_device_changes_only_its_own_row` |
| `signature.rs:95:22` `>` por `<` en `write_canonical` | Detectado | `signature::tests::the_items_of_an_array_are_separated_by_commas` (las comas entre elementos de un array; los casos de `signatures.json` solo llevan un cambio) |

## Hallazgo

Al intentar matar `sync.rs:293` apareció un fallo real, no tocado aquí: dos operaciones firmadas por el mismo dispositivo con el mismo HLC y distinto id hacen que `merge` devuelva `Err(Sqlite(ConstraintViolation: UNIQUE constraint failed: operations.hlc))` y se aborte todo el lote, cuando `merge` promete no fallar por el contenido de lo que recibe. Un compañero con la app modificada (o un reloj duplicado) puede dejar sin sincronizar a quien le reciba: el lote se reenvía y falla cada vez. **Arreglado el mismo día:** en la app, `sync::hlc_taken` rechaza la segunda operación y retiene las siguientes de su autor (test `two_operations_with_one_hlc_do_not_break_the_merge`); en el servidor, `CloudPush` la rechaza con `hlc_taken` y un índice único `(cloud_library_id, hlc)` lo impide también en la base de datos (test `testASecondOperationWithAnHlcAlreadyStoredIsRefused`). Los dos tests se vieron fallar antes del arreglo. El mutante `sync.rs:293` sigue siendo equivalente: ahora la operación se rechaza antes de llegar ahí.

## Para `spec/` en 7.4.2

- `spec/conformance/library-merge.json`: un cambio con el mismo HLC que la versión guardada de un campo no lo sobrescribe (empate no es más nuevo); un cambio que pierde en un campo y gana en otro deja intacta la versión del campo perdido; reloj de un compañero más de una hora por delante (justo una hora sí, una hora y un milisegundo no); regla del contador de `receive` (mayor contador cuando empatan los milisegundos, el del reloj que decide en los demás casos).
- `spec/conformance/permissions.json`: un dispositivo solo cambia su propia fila (otro dispositivo es de administradores); una operación que queda como solicitud no deja huella de su vallado; «misma persona» (el mismo dispositivo, o dos dispositivos del mismo miembro, y nunca dos dispositivos sin miembro distintos).
- `spec/conformance/signatures.json`: un caso con varios cambios, para fijar las comas entre elementos del array en los bytes canónicos. Y, con estado de biblioteca, o en `permissions.json`: la clave de un dispositivo no registrado solo vale si la trae su propio `Create`; un `Update` de sí mismo o de otro id no la registra; un dispositivo registrado no cambia ninguna clave, ni la suya.
- `spec/conformance/operation-validation.json`: forma de la intención (dos o más palabras en minúsculas separadas por puntos, sin vacías ni mayúsculas) y forma del texto del HLC (16 dígitos, 8 hexadecimales, dispositivo no vacío, cada parte por separado).
- Solo si el servidor valida el nombre de una biblioteca: 60 caracteres es válido y 61 no.
- Sin caso en `spec/`: el reparto entre fusión incremental y reconstrucción (`judged_by_order`, `note_fences`, `newest_hlc`), los contadores del `MergeReport` y `fences_of`/`replay`; son detalles del cliente. Lo que el servidor tiene que cumplir es que el resultado sea el mismo, y eso ya lo cubre `assert_rebuild_changes_nothing`.
