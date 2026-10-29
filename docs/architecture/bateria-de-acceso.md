# Batería de acceso

> Documento vivo: la lista de tests de casos hostiles sobre **quién recibe una biblioteca** (emparejar, compartir, retirar el acceso, la tabla de acceso de la nube). Nació en el plan de la Fase 7 ([historico/fase-7.md](../historico/fase-7.md)) y se sacó de él al cerrar la fase (25/10/2026), porque sigue creciendo: cada regla nueva sobre quién recibe una biblioteca suma aquí su caso hostil, escrito antes del arreglo y visto fallar (la regla está en `AGENTS.md`). Lo siguiente que la espera: emparejar en casa sin QR (D88).

Añadida el 03/10/2026 (D84). La primera línea de defensa son estos tests; la auditoría `library-access` es la segunda. Cada test se escribe antes que su arreglo, se ve fallar y después pasar. Toda regla nueva sobre quién recibe una biblioteca añade aquí su caso hostil ([AGENTS.md](../../AGENTS.md)).

**Red local (Rust, por TLS de verdad, en `lan/client.rs`):**

| Test | Qué comprueba | Estado |
|---|---|---|
| `a_deleted_library_is_not_taken_up_when_offered` | SEG-010: quien borró una biblioteca no la acepta sola cuando otro le ofrece el mismo id, y su borrado no llega al otro | ✅ arreglado (`libraries::library_known`) |
| `a_forged_revocation_notice_removes_nothing` | SEG-011: un aviso de retirada sin la retirada firmada no borra nada | ✅ arreglado (el aviso lleva la retirada firmada y se juzga) |
| `a_library_both_have_travels_only_if_both_share_it` | El id fijo solo se funde si los dos lados lo marcan | ✅ |
| `only_the_chosen_libraries_travel` | Nada sale sin marcarlo | ✅ |
| `a_revoked_device_is_refused_its_old_offer` | Un retirado no vuelve a meter cambios | ✅ |
| `a_revocation_reaches_the_device_through_another_one` | La retirada llega por un tercero | ✅ |
| `a_paired_device_cannot_pair_again_until_forgotten` | Nadie ocupa el sitio de un dispositivo emparejado | ✅ |
| `a_wrong_number_pairs_nothing` | D88: con un número de 6 cifras equivocado no se empareja y ninguno de los dos guarda nada | ✅ |
| `three_wrong_numbers_close_the_window` | D88: a los tres intentos la ventana se cierra, también para el número bueno | ✅ visto en rojo (sin límite) y arreglado |
| `a_device_in_the_middle_gets_nothing_from_the_joining_one` | D88: quien se pone en medio y pasa la respuesta del otro (número incluido) sirve con otro certificado; su confirmación no cuadra con la huella que ve el que se une, que no le manda nada | ✅ visto en rojo (el cliente no comprobaba la confirmación) y arreglado |
| `a_number_pairs_one_device_only` y `the_number_does_not_take_the_place_of_a_paired_device` | D88: el número es de un solo uso y no ocupa el sitio de un dispositivo emparejado | ✅ |
| `only_a_device_with_its_window_open_is_found_and_only_by_name` | D88: buscar dispositivos para emparejar por número solo encuentra a quien tiene la ventana abierta, y antes del número solo se lee su nombre; uno mismo no sale | ✅ |
| `a_demoted_editor_cannot_backdate_a_change` (en `sync.rs`) | SEG-014: a quien bajaron de editor a lector no puede firmar después un cambio con fecha anterior a la bajada | ✅ arreglado con las marcas de D84 (y `an_offline_edit_made_before_hearing_of_the_demotion_becomes_a_request`, `a_revoked_device_cannot_backdate_a_change_either` y cinco casos en `permissions.json`) |
| `a_revoked_device_s_edit_relayed_by_a_third_device_does_not_apply` y `without_roles_…` (en `sync.rs`), y un caso en `permissions.json` | SEG-015: lo que firma un dispositivo retirado después de la retirada no entra, aunque llegue por otro que no lo sabía, con roles y sin ellos | ✅ arreglado (la retirada es una marca abierta) |
| `a_library_left_here_is_not_downloaded_again` y `a_library_removed_by_a_revocation_is_not_downloaded_again` (en `cloud/tests.rs`) | SEG-017: quien dejó una biblioteca o fue retirado de ella no la vuelve a bajar de la nube (`deleted_here`) | ✅ arreglado (`forgotten_libraries`; el hito 7.5 cambia cómo sale de la cuenta el retirado) |
| `adopting_a_link_brings_what_the_author_wrote_before_it` (en `cloud/tests.rs`) | DAT-002: al adoptar un enlace llega todo lo que el autor escribió antes de subir, sin saltos en su vector | ✅ arreglado (`adopt_link` fusiona todo) |
| `a_rejected_operation_holds_back_the_later_ones_of_its_author` (en `sync.rs`) | DAT-004: una operación rechazada no deja que el vector salte sobre ella | ✅ arreglado |
| `a_device_paired_after_an_app_update_gets_the_old_history` (en `sync.rs`) | DAT-003: un dispositivo emparejado tras una actualización recibe el historial de la versión anterior | ✅ arreglado (D31 precisado) |
| `an_operation_whose_previous_is_missing_waits_for_it` (en `sync.rs`) | D87: una operación sin su anterior no se aplica, el vector del autor se queda por debajo y la siguiente sincronización trae las dos | ✅ |
| `a_previous_that_is_not_an_earlier_operation_of_the_same_author_and_library_is_refused` (en `sync.rs`) | D87: una cadena que apunta a otro autor, a otra biblioteca o a una operación no anterior se rechaza; la honrada pasa | ✅ |
| `a_fork_dated_below_the_fence_is_a_request_whatever_the_order` (en `sync.rs`) y el caso `an operation dated below the mark and chained to an older one is a request (SEG-018)` de `permissions.json` | SEG-018: quien perdió el rol firma con una app modificada una operación fechada por debajo de su marca, colgada de una anterior suya: petición en cualquier orden y en todos los dispositivos | ✅ cerrado con D87 |
| `a_fence_that_arrives_before_its_operation_is_judged_again_when_it_arrives` (en `sync.rs`) | D87: si la operación marcada aún no está, la valla cubre todo lo del dispositivo; cuando llega se recalcula y lo anterior vuelve a contar | ✅ |
| `a_restored_device_writes_a_new_chain_and_syncs_with_peers_that_hold_the_old_one` (en `backup.rs`) y `a_device_that_left_a_library_and_took_it_up_again_continues_its_own_chain` (en `sync.rs`) | D87: restaurar una copia o dejar y recuperar una biblioteca no rompe la cadena ni se atasca | ✅ |
| `a_long_log_is_pushed_in_an_order_where_every_previous_goes_first` (en `cloud/tests.rs`) | D87: la subida de un registro largo respeta la cadena entre lotes | ✅ |
| `a_stranger_cannot_date_a_takeover_before_the_activation` (en `sync.rs`) y los casos 1, 2, 5, 6 y 7 de «sello» de `permissions.json` | SEG-025 (D94): un dispositivo que quien activa no conoce no puede fechar antes de la activación una operación que le haga administrador, ni cambiar la fila de la biblioteca, ni poner un «sello» propio antedatado | ✅ visto en rojo y arreglado con el sello |
| `turning_roles_on_seals_the_library_naming_every_device_it_knows`, `a_library_with_roles_and_no_seal_is_sealed_once`, `a_device_that_was_away_seals_after_merging_and_keeps_what_the_others_did`, `a_seal_written_before_merging_leaves_what_the_others_did_as_requests`, `a_device_that_is_not_an_administrators_does_not_seal` y `two_administrators_who_seal_at_once_…` (en `sync.rs`) | D94: quién sella, el sellado único tras fusionar en una sincronización (no al arrancar), y que dos sellos a la vez convergen | ✅ |
| `a_vouched_key_that_is_not_the_registered_one_makes_the_device_nobodys_in_every_order` (en `sync.rs`) y los casos de «clave avalada» de `permissions.json` | D96 (SEG-019): un dispositivo asignado a una persona con una clave que no es la suya es de nadie, en cualquier orden de llegada, y no cambia un rol aunque lo hayan asignado a una administradora | ✅ visto en rojo (los casos 2, 3, 5, 6 y 7 de «clave avalada» fallaban sin la regla) |
| `an_editor_who_may_share_cannot_upload_and_the_server_receives_nothing` (en `cloud/tests.rs`) | CON-009: «Puede compartir» no sube a la nube; solo quien administra (o cualquiera sin roles) | ✅ |

**Nube (PHP, en `server/tests/`, contra las rutas de 7.2; se escriben al empezar 7.2, en rojo).** Todos con dos cuentas, la de una familia y la de **Jhonny**, que es hostil y conoce los ids de la familia:

| Test | Qué comprueba |
|---|---|
| Mismo id local, dos bibliotecas | La familia y Jhonny suben bibliotecas con el mismo id local (el fijo): salen dos `C` y ninguna operación cruza |
| Sin acceso es como no existir | Jhonny pide `pull`/`push` de la `C` de la familia: `404`, sin datos, sin escribir nada |
| Ids de operación robados | Jhonny sube a su `C` operaciones con los ids de la familia antes que ella: las de la familia entran igual en su `C` |
| Id local ajeno | Jhonny intenta meter en su `C` operaciones de otro id local, o la familia en la suya: rechazadas |
| Sin enlace no hay nada | Un `push` a una `C` recién creada sin la operación de enlace firmada por un dispositivo de la cuenta: rechazado |
| Firma con clave ajena | Jhonny registra en su `C` una fila de dispositivo con el id de un móvil de la familia: en la `C` de la familia sus firmas no valen |
| Retirado y quitado | Un dispositivo quitado (D81) o retirado (D71) deja de recibir al momento |
| Sin cuenta, sin entrada | Un hijo sin cuenta no aparece en la tabla de acceso y sigue por la red local; lo suyo llega a la nube por su padre |
| No rechazar por orden | La operación de un autor llega después que otra más nueva **de otro autor**: se guarda (DAT-001). Con D87, la de un mismo autor espera a su anterior |
| Esperar a la anterior (D87) | `testAnOperationWhoseCloudLacksThePreviousWaitsForIt`: un `push` de a2 sin a1 no guarda nada de ese autor y su vector queda por debajo; con a1 y a2 se guardan las dos |
| Cadena mal formada (D87) | `testAPreviousThatIsNotAnEarlierOperationOfTheSameAuthorIsRefused`: `previous` de otro autor o no anterior se rechaza; `testAnOperationWithoutThePreviousKeyIsRejected`: sin la clave no se lee |
| Nadie ocupa el id de la nube (D95) | `CloudSyncControllerTest`: un miembro con acceso no registra el id de la nube con una clave suya, ni firma con él (rechazadas, la clave de la nube no cambia); un enlace que nombra a la nube con otra clave no enlaza la biblioteca; la API no devuelve la clave privada ni cifrada y la base no la guarda en claro; quien no tiene acceso no ve el id de la nube |
| La nube firma (D95) | `CloudAuthorTest`: se registra al final del `push` que la nombra (por lotes, con la ruta de enlace o de una vez) y una sola vez; lo que firma queda aplicado, encadenado, con su firma verificable y repartido por `pull`; sin nombrarla, en petición; con el reloj parado, HLC distintos y en orden; `CloudKeyVaultTest`, `OperationSignerTest` y `CloudWithoutSecretTest` (secreto vacío) |
| Quien ocupa un id no hereda a su persona (D96) | `CloudSyncControllerTest::testAnAdministratorVouchingForTheRealKeyOfAnIdJhonnyHadRegisteredLeavesHimNothing`: Jhonny, con acceso, registra en la nube el id de un dispositivo nuevo con una clave suya; la administradora asigna ese id a su persona avalando la clave de verdad; lo que Jhonny firma como ese dispositivo (un cambio de rol) queda en petición y el rol no cambia |
| El enlace exige la clave avalada (D96, 7.5.2b-2) | `AccountLinkerTest`: el ataque de D96 (Jhonny con acceso registra el id de la tablet de Laura con su clave, ella la asigna avalando la buena, él llama a `claim`: no queda enlazado, su fila no cambia y `account_id` de Laura no es el suyo); Jhonny sin acceso con el id del móvil de Laura: nada y `404`; asignación sin clave avalada, dispositivo de nadie, lector que se asigna a sí mismo (petición), dispositivo con retirada, persona que ya tiene otra cuenta, cuerpo con ids (ignorado), biblioteca sin enlazar, y la cuenta enlazada como lector que lee pero lo que sube es una petición |
| El móvil nuevo (7.5.3) | `AccountLinkerTest`: un móvil al que un administrador quitó su persona con valla, o con una retirada, no lo vuelve a vincular la nube; Jhonny, que registró antes el id del móvil nuevo de Hugo con una clave suya, no consigue que se vincule nada; una cuenta con fila pero sin persona no vincula; el móvil nuevo de un lector queda lector y lo que sube es petición; la nube sin poder no escribe nada; un dispositivo que ya es de otra persona no se toca; un cuerpo con ids se ignora; dos llamadas, una sola operación. Y los casos de la regla 12 en `permissions.json` |
| Echar a una persona (D98, 7.5.4a) | Los casos «D98» de `permissions.json` (en rojo antes de la regla: la marca sola, quién la escribe, la nube que vincula a una persona echada, devolver el acceso, echar a una administradora) y, en Rust, `withdrawing_a_person_removes_the_library_from_their_device_when_it_syncs` (el echado borra la biblioteca por el camino de D71), `what_the_expelled_signs_after_the_withdrawal_is_a_request_in_every_order`, `the_last_administrator_and_oneself_are_not_withdrawn_and_the_withdrawn_do_not_count` |
| La nube corta el acceso (D98, 7.5.4b) | `AccountLinkerTest`: un dispositivo retirado recibe `403` con la prueba y nada más mientras el otro dispositivo de la cuenta sigue entrando; echar a Jhonny corta todos sus dispositivos y uno nuevo (listado, `claim` y `events` incluidos); la nube vacía su `account_id` (firmado, una vez; sin poder no escribe); quitar la marca no devuelve nada y volver es vincular de nuevo; su app modificada no puede quitarse la marca ni borrar su retirada; el mismo id con otra clave no se corta; quien nunca tuvo la biblioteca recibe `404`; lo que manda el cliente no cambia la respuesta; la retirada que llega tarde corta igual. En la app: `a_server_that_lies_about_the_access_removes_nothing` (sin pruebas, de otra biblioteca, firmada por un lector, de otro dispositivo: la biblioteca se queda entera) y `a_phone_of_a_person_who_was_withdrawn_removes_the_library_with_the_proof_and_it_does_not_come_back` |

**Las dos a la vez (Rust, simulando dispositivos y un servidor de prueba):** un dispositivo edita una semana sin conexión y luego sincroniza con la nube y con un compañero de la red local, en los dos órdenes: mismo estado final, nada duplicado, nada perdido (`assert_converged`). Una retirada que llega solo por un camino.

**Escenarios de la familia (Ana administra, Hugo lee):** Ana nunca conecta con la nube y Hugo inicia sesión (la subida de Hugo no conecta a nadie); Ana sube; Ana cierra sesión una semana mientras Hugo sigue (desde 7.5). Se escriben con 7.2 y 7.5.

**La papelera (D105, regla 16 de `spec/README.md`)**, en `spec/conformance/permissions.json` (los casos que empiezan por «trash:»), que corren la app y el servidor. No es quién recibe una biblioteca, pero da a la nube un poder nuevo sobre el catálogo, y por eso está aquí. Escritos antes que la regla el 27/10/2026; los marcados se vieron fallar con el juez de entonces, que dejaba mandar a la papelera lo ajeno, falsear quién lo mandó y borrar sin papelera:

| Caso | Qué comprueba | Estado |
|---|---|---|
| `an editor sends to the trash what she may delete; restoring is for any editor` | Mandar a la papelera sigue la regla de borrar (lo de su persona); restaurar, cualquier editor | ✅ visto en rojo y arreglado |
| `the person who sends to the trash is the signer's, an administrator's included` | `trashed_by` se compara con la persona de quien firma, también si administra (D84) | ✅ visto en rojo y arreglado |
| `a reader neither sends to the trash nor restores` | Quien solo lee, nada: petición | ✅ |
| `an editor deletes for good only after 30 days in the trash, whoever created it; never without the trash` | Un editor no destruye nada antes de 30 días, ni lo suyo sin pasar por la papelera | ✅ visto en rojo y arreglado |
| `an administrator deletes for good at any moment, in the trash or not` | Quien administra, como siempre | ✅ |
| `what hangs from a book in the trash goes with it after 30 days, whoever created it` | Ejemplares, detalles, obra y enlaces se van con la ficha, no antes | ✅ visto en rojo y arreglado |
| `a series in the trash takes its entries along after 30 days; the works stay` | Las entradas cuelgan también de su saga; la obra no se va con ella | ✅ |
| `restoring a copy brings it back from the trash; one restored is not deleted for good by an editor` | Lo restaurado ya no cuenta los 30 días | ✅ |
| `a row created already in the trash, as restoring a backup does, names a person of the library` | Al restaurar una copia de seguridad, `trashed_by` puede ser otra persona, pero de la biblioteca | ✅ |
| `the cloud deletes for good what has been 60 days in the trash, and nothing else` | El único poder de la nube sobre el catálogo: antes de 60 días, fuera de la papelera, mandar o restaurar, petición | ✅ |

Riesgo aceptado (D105): una nube comprometida podría fechar su operación a su favor y eliminar antes de tiempo lo que ya estaba en la papelera. El servidor todavía no corre estos casos: es el paso 3 del [plan](../historico/papelera.md).

**Casos hostiles de las invitaciones** (7.6, en `server/tests/Controller/InvitationControllerTest.php`; cada uno con su test antes de la defensa, visto fallar):

1. Un enlace usado, caducado o anulado no da nada, y dos cuentas que lo aceptan a la vez: entra una.
2. Una invitación por correo aceptada por una cuenta con otro correo.
3. Quien no administra crea una invitación con una app modificada (lector, editor, «Puede compartir»).
4. Quien administra la biblioteca A crea una invitación con el id de una persona de la biblioteca B: el servidor compara, no busca por el id que le mandan (D84).
5. Un enlace para copiar creado para una persona que no es nueva o no es lectora (una app modificada): se comprueba al crear **y al aceptar**.
6. Una invitación para una persona que ya administra, que tiene el acceso retirado (D98) o que ya tiene una cuenta enlazada.
7. Quien invitó deja de administrar, o es echado, antes de que se acepte: la invitación ya no vale.
8. Una cuenta que ya es otra persona en esa biblioteca acepta una invitación.
9. Adivinar enlaces: el token es largo y aleatorio, solo se guarda su huella, y la ruta tiene límite de peticiones.
10. Usar el servidor para mandar correo a direcciones ajenas: límite de invitaciones por correo por cuenta y día, y un texto que no lleva nada escrito por quien invita salvo el nombre de la biblioteca.
