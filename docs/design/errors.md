# Catálogo de errores, avisos y ayuda

> Cada problema que la app puede mostrar: qué ve el usuario, por qué pasa y qué debe hacer. Sirve al diseño (qué mensajes y ayudas hay que dibujar) y a la implementación (la columna **Código** es el identificador que envía la app). Patrón y niveles, en el [brief](brief.md#9-errores-avisos-y-ayuda-muy-importante).
>
> Niveles: **info** (neutral) · **aviso** (ámbar, no se perdió nada) · **error** (rojo, la acción no se hizo).

## Formularios (alta, edición, deseos)

Van debajo de cada campo, en rojo; el formulario no se cierra.

| Código | Mensaje | Qué hacer (texto de ayuda) |
|---|---|---|
| `required` | Este campo es obligatorio. | Escribe al menos el título; lo demás se puede completar después. |
| `isbn_length` | Un ISBN tiene 10 o 13 dígitos. | Cópialo de la contraportada o de la página de créditos. Los guiones y espacios no importan. |
| `isbn_character` | Un ISBN solo lleva números (y a veces una X al final). | Revisa que no se haya colado una letra; la «X» solo puede ir al final de un ISBN de 10. |
| `isbn_checksum` | Este ISBN no es válido. | Suele ser una errata: comprueba cada dígito, o escanéalo con la cámara. |
| `isbn_in_use` | Otro libro del catálogo ya tiene este ISBN. | Si es el mismo libro, añade un ejemplar desde su ficha en vez de cambiar este. **Botón:** «Abrir ese libro». |
| `out_of_range` | Este valor está fuera de rango. | Año entre 1 y 2100; unidades entre 1 y 999. |
| `invalid_date` | No es una fecha válida. | Elige la fecha en el calendario. |
| `invalid_cover_url` | No se puede usar esta portada. | Busca de nuevo por ISBN para tomar la portada de Google Books u Open Library. (Por seguridad solo se descargan portadas de esos servicios.) |

## Búsqueda de datos online

Son **avisos**, nunca errores: el alta sigue siendo posible a mano.

| Código | Mensaje | Qué hacer |
|---|---|---|
| (sin resultados) | No está en los catálogos online. | Rellénalo a mano; solo el título es obligatorio. Pasa con ediciones españolas poco comunes o muy antiguas. |
| `quota` (Google Books) | Google Books no ha respondido (límite de consultas). | Es un límite de Google para consultas sin clave. Para mejores resultados, crea una clave gratuita y ponla en Ajustes. **Botón:** «Cómo conseguir una clave». Mientras, se usan los datos de Open Library. |
| `network` | Sin conexión: no se pueden buscar datos online. | Revisa la conexión, o rellénalo a mano y búscalo más tarde desde «Editar». |
| `unexpected` | {Servicio} ha respondido algo inesperado. | Inténtalo más tarde; si se repite, puede que el servicio haya cambiado. |
| `cover_failed` | Libro guardado, pero sin portada. | La portada no se pudo descargar. Vuelve a buscar por ISBN desde «Editar» cuando haya conexión. |

## Escáner y cámara

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `camera_denied` | error | La app no tiene permiso para usar la cámara. | Ajustes del móvil → Aplicaciones → Library → Permisos → Cámara → Permitir. **Botón:** «Abrir ajustes». También puedes teclear el ISBN. |
| (cancelado) | — | *(nada: el usuario cerró la cámara)* | — |
| (código que no es un libro) | aviso | Eso no parece el código de un libro. | Busca el código de barras que empieza por 978 o 979, normalmente en la contraportada. Si no lo tiene (libros muy antiguos o de rol independientes), añádelo a mano. |

## ¿Lo tengo ya?

| Situación | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| Sin cobertura y no está en el catálogo | info | No está en vuestro catálogo. Sin conexión no se pueden ver sus datos. | Apúntalo en deseos con un título; los datos se completan cuando lo pases al catálogo con conexión. |
| Hace mucho que no se sincroniza | aviso | Este móvil no se sincroniza desde hace {n} días: puede que falten libros añadidos en otros dispositivos. | Abre la app en casa, en la misma wifi que el portátil. **Botón:** «Sincronizar ahora». |

## Copias de seguridad y CSV

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `not_a_backup` | error | Este fichero no es una copia de seguridad de la app. | Elige un `.zip` o `.kosmotheka` creado con «Guardar copia». Un CSV se carga con «Importar CSV». |
| `unreadable` | error | La copia está dañada y no se puede leer. | Prueba con otra copia más antigua. Si la has pasado por correo o por la nube, descárgala de nuevo. |
| `old_encrypted_backup` | error | Esta copia se hizo con una versión de desarrollo que usaba otro formato y no se puede abrir. | Solo las versiones de desarrollo del 3 de octubre de 2026 las crearon. Haz una copia nueva desde la app. |
| `other_version` | error | Esta copia se hizo con otra versión de la app. | Actualiza la app a la última versión en este dispositivo e inténtalo de nuevo. |
| `not_empty` | error | Solo se puede restaurar en un catálogo vacío. | Restaurar reemplazaría lo que tienes. Si quieres **juntar** catálogos, empareja los dispositivos en «Dispositivos». Si de verdad quieres restaurar, hazlo en una instalación nueva. |
| `passphrase_needed` | error | Esta copia está protegida: escribe su contraseña. | La copia es un fichero cifrado (D86); la pantalla pide la contraseña. |
| `wrong_passphrase` | error | Contraseña incorrecta. No se ha restaurado nada. | Prueba otra vez. Si la has perdido, la copia no se puede abrir: no hay forma de recuperarla. |
| `passphrase_empty` | error | Escribe una contraseña para la copia. | O desmarca «Proteger con contraseña» sabiendo que la copia quedará legible para quien la encuentre. |
| (filas rechazadas en CSV) | aviso | Importados {n} libros; {m} filas no se pudieron importar. | Revisa las líneas indicadas (cada una dice qué campo falla), corrígelas en la hoja de cálculo e importa solo esas filas de nuevo. |
| `invalid_format` (CSV) | — | Formato desconocido en la línea {n}. | Usa: hardcover, paperback, pocket, ebook, audiobook u other (o déjalo vacío). |

## Dispositivos y sincronización

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `bad_code` | error | Eso no es un código de emparejamiento. | Copia el código completo (empieza por `KOSMOTHEKA1:`), o mejor, escanea el QR. |
| `same_device` | error | Ese código es de este mismo dispositivo. | El código se muestra en un dispositivo y se escanea o pega en **otro**. |
| `unreachable` (al emparejar) | error | No se encuentra el otro dispositivo. | Ver la guía **«El móvil no se empareja»** (abajo). **Botón:** «Ver qué revisar». |
| `unreachable` (al sincronizar) | aviso | No se pudo sincronizar con {nombre}: no está en la red. | Normal si estás fuera de casa o el otro está apagado; se sincronizará solo cuando coincidáis. Si estáis en la misma wifi, ver la guía. |
| `rejected` (al emparejar) | error | El código ya no es válido. | Los códigos valen **una vez** y **10 minutos**. En el otro dispositivo, pulsa «Mostrar código» otra vez. **Botón:** «Escanear otro código». |
| `rejected` (al sincronizar) | error | {nombre} ya no reconoce este dispositivo. | Alguien lo olvidó en el otro dispositivo, o se reinstaló la app allí. Olvídalo aquí y empareja de nuevo. **Botón:** «Emparejar de nuevo». |
| `wrong_certificate` | error | El dispositivo que ha respondido no es el esperado. | Por seguridad no se ha conectado. Si reinstalaste la app en el otro dispositivo, su identidad ha cambiado: olvídalo y empareja de nuevo. Si no, puede que otro equipo de la red esté usando esa dirección. |
| `other_version` | error | Los dos dispositivos tienen versiones distintas de la app. | Actualiza la app en ambos a la misma versión y sincroniza de nuevo. |
| `no_network` | error | Este dispositivo no está conectado a ninguna red. | Conéctalo a la wifi de casa para mostrar el código. |
| `peer_error` | error | El otro dispositivo ha tenido un problema. | Inténtalo de nuevo. Si se repite, reinicia la app en los dos dispositivos. |
| `unknown_device` | error | Este dispositivo ya no está emparejado. | Empareja de nuevo desde «Dispositivos». |

### Guía: «El móvil no se empareja» (o no sincroniza en casa)

Una pantalla de ayuda con comprobaciones, de la más probable a la menos. Cada una, con su explicación breve y, si es posible, un botón.

1. **¿Están en la misma wifi?** El móvil debe estar en la wifi de casa, no con datos móviles. Algunas casas tienen dos redes (por ejemplo, `Casa` y `Casa-5G`): normalmente se ven entre sí, pero no siempre.
2. **¿Es una red de invitados?** Las redes de invitados (y algunas de oficinas, hoteles o bibliotecas) aíslan los dispositivos entre sí a propósito. Usa la red principal.
3. **¿Está la app abierta en el otro dispositivo, con el código a la vista?** El código solo funciona mientras se muestra, durante 10 minutos, y una sola vez.
4. **¿Hay una VPN activa?** Una VPN en el portátil o en el móvil puede impedir que se vean en la red local. Desactívala mientras emparejas, o busca en ella la opción «permitir la red local» (en algunas se llama «LAN discovery»).
5. **¿El cortafuegos lo bloquea?** En Windows, la primera vez aparece un aviso del cortafuegos: permite el acceso en **redes privadas**. Si lo rechazaste: Configuración → Privacidad y seguridad → Seguridad de Windows → Firewall → Permitir una aplicación. En Linux, si usas un cortafuegos (`ufw`, por ejemplo), permite el puerto que muestra «Dispositivos».
6. **¿El móvil pide permiso de «red local» o «dispositivos cercanos»?** Algunas versiones de Android lo piden para conectar con equipos de casa: acéptalo.
7. **¿Es un router con «aislamiento de clientes»?** Algunos routers o sistemas wifi mesh lo traen activado. Se desactiva en la configuración del router.
8. **Último recurso:** reinicia la app en los dos dispositivos y genera un código nuevo.

*(Detalles técnicos plegados, para copiar si hay que pedir ayuda: direcciones que se probaron, puerto, versión de la app.)*

## Cuenta y nube (Fase 7)

Propuestos por Design el 02/10/2026 ([pantallas](screens/cloud-account/), [original](screens/cloud-account/errors-fase-7.md)). La segunda columna es el código que devuelve el servidor ([architecture/accounts.md](../architecture/accounts.md)); la app lo traduce a este texto. Inglés en el original.

| Código | Servidor | Nivel | Mensaje | Qué hacer |
|---|---|---|---|---|
| `code_incorrect` | `invalid_code` | error | Código incorrecto. | Usa el del último correo que te ha llegado. |
| `code_expired` | `code_expired` | error | Este código ha caducado. | Los códigos valen 15 minutos. Pide otro y úsalo en cuanto llegue. **«Enviar otro código»** |
| `code_attempts` | `too_many_attempts` | error | Demasiados intentos con este código. | Ya no sirve. Pide otro: llega en un momento. **«Pedir otro código»** |
| `rate_limited` | `too_many_requests` (con `Retry-After`) | aviso | Demasiadas peticiones. | Espera {n} minutos e inténtalo de nuevo. |
| `bad_credentials` | `invalid_credentials` | error | El email o la contraseña no son correctos. | Revisa cómo está escrito el email. Si no recuerdas la contraseña, pon una nueva. **«Recuperar la contraseña»** |
| `email_unconfirmed` | `account_not_confirmed` | aviso | Falta confirmar tu email. | Al crear la cuenta te enviamos un código. Si no lo encuentras, pide otro. **«Enviar otro código»** |
| `account_locked` | `account_blocked` | error | Esta cuenta está bloqueada. | Si crees que es un error, escribe a quien gestiona el servidor. |
| `totp_incorrect` | `invalid_two_factor_code` (con el código de la app) | error | El código no es correcto. | Usa el que se ve ahora en la app: cambia cada 30 segundos. Comprueba que la hora del móvil sea automática. **«Usar un código de respaldo»** |
| `backup_incorrect` | `invalid_two_factor_code` (con un código de respaldo) | error | Ese código de respaldo no vale. | Puede que ya lo usaras: cada uno vale una vez. Prueba con otro de la lista. |
| `session_revoked` | `signed_out_password_changed` | info | Se ha cerrado la sesión. | La contraseña se cambió en otro dispositivo. No se pierde nada: los cambios sin subir se enviarán al volver a entrar. **«Iniciar sesión»** |
| `device_removed` | `device_removed` | info | Este dispositivo se quitó de una cuenta. | Se quitó desde otro dispositivo y sus bibliotecas se han borrado. La app está lista para usar, con o sin cuenta. |
| `cloud_offline` | (sin respuesta) | aviso | Sin conexión con la nube. | Todo sigue funcionando aquí y por la red de casa; los cambios se subirán cuando vuelva la conexión. |
| `server_unavailable` | (sin respuesta o 5xx) | aviso | El servidor no responde. | Puede estar en mantenimiento, o la dirección no es correcta. Inténtalo en unos minutos. **«Reintentar»** |
| `invite_invalid` | (hito 7.6) | error | Esta invitación ya no vale. | Ya se usó o han pasado más de 24 horas. Pide a {nombre} que te mande otra. |
| `password_short` | 422 de validación | error | La contraseña necesita al menos 8 caracteres. | |
| `not_found` | `not_found` | error | Algo ha fallado. | Inténtalo de nuevo. Si se repite, avisa. |
| `invalid_request` | `invalid_request` | error | Algo ha fallado. | Inténtalo de nuevo. Si se repite, avisa. |
| `schema_version` | `schema_version` | aviso | Esta versión de la app no puede sincronizar con el servidor. | Actualiza la app e inténtalo de nuevo. |

- Inicio de sesión y códigos: en un bloque bajo los campos; el formulario no se cierra.
- `session_revoked` y `device_removed`: pantalla propia al abrir la app.
- `cloud_offline`: indicador discreto (barra superior en escritorio, cabecera de Compartir en móvil) y fila de la nube; nunca bloquea.
- `invite_invalid`: pantalla propia al abrir el enlace.
- El servidor no dice cuántos intentos quedan ni si un email ya tiene cuenta: crear cuenta siempre pasa a la pantalla del código, y si el email ya tenía cuenta le llega un correo avisando.

## La cuenta dentro de la biblioteca (Fase 7, 2.ª tanda)

Propuestos por Design el 10/10/2026 ([original](screens/cloud-family/errors-fase-7b.md)); aquí solo los que la app muestra, con el código **real** (los nombres de Design no valen). Construidos en el hito 7.5.5b.

| Código de Design | Código real | Nivel | Dónde y cómo |
|---|---|---|---|
| `access_revoked_wiped` | resultado `removed` de la sesión con la nube (la retirada comprobada) | info | Aviso tras sincronizar (`cloud.notice.removed`), con el nombre de la biblioteca que Rust guarda en el informe. Design pedía una pantalla propia con «Ir a mis bibliotecas»: la app lo dice con un aviso. El texto junta también `device.removed_by_admin` |
| `access_revoked_unverified` | resultado `access_withdrawn` de la sesión | aviso | Aviso tras sincronizar (`cloud.notice.accessWithdrawn`); la biblioteca sigue aquí y deja de sincronizarse con la nube |
| `upload_admin_only` | `link_not_allowed` (403 del servidor al enlazar) o `not_allowed` (la app, antes de llamar) | info | `accountErrors.linkNotAllowed` y `cloud.errors.link_not_allowed`. Sin «Pídeselo a {nombre}»: la app no sabe a quién |
| — (sin equivalente en Design) | `link_rejected` | error | `accountErrors.linkRejected`: el servidor rechaza el enlace firmado de la biblioteca |
| — | `events_unavailable` | info | `accountErrors.eventsUnavailable`: el servidor no tiene avisos al momento |
| — | `access_withdrawn` (error, `403` con pruebas) | aviso | `accountErrors.accessWithdrawn`, `cloud.errors.access_withdrawn` |
| — | `no_cloud_device` (solo de la app) | error | `cloud.errors.no_cloud_device`: el servidor no da identidad a la nube y no se puede dar ese poder |

Sin código real (no se inventan; comprobado en 7.5.5c, 10/10/2026): el servidor no da ningún código en estos tres casos, así que no llegan a la app.
- `device_duplicate`: un dispositivo registrado con otra clave simplemente no se vincula (`AccountLinker::linkDeviceIn` devuelve falso, sin código).
- `person_account_taken`: `AccountLinker::link` devuelve `null` en silencio cuando otra cuenta ya es esa persona; `claim` no lo cuenta.
- `cloud_link_disabled`: no hay error; el interruptor solo vacía `cloud_device_id` en la biblioteca, y la nube sin poder no escribe nada. Sería un texto inferido, no un error del servidor.

Lo que sí llega es `claim.conflicts` («este dispositivo es de otra persona»): diálogo `cloud.otherOwner.*`. Los textos nuevos que no son errores (`cloud_pick.*`, `joined.*`, `autolink.*`, `device.no_person`) están en las claves `cloud.pick.*`, `cloud.notice.joined*`, `cloud.autolink.*` y `people.noPerson*` de los ficheros de idioma.

## Errores internos

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `internal` | error | Algo ha fallado y no se ha podido completar. | Inténtalo de nuevo. Si se repite, copia los detalles (plegados) y avisa. **Nunca** mostrar el texto técnico como mensaje principal. |

## Pendiente de implementar

Hoy la app muestra estos mensajes de forma más escueta: la mayoría sin el «qué hacer», sin botones de acción y sin la guía de emparejamiento. También faltan situaciones que aún no detecta: el código de barras que no es de un libro, «hace mucho que no se sincroniza» y el aviso de sin conexión. Esta tabla es la referencia para implementarlos cuando llegue el nuevo diseño.
