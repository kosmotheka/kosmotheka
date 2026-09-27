# Plan de la Fase 5 — Personas, roles y peticiones

> Plan de trabajo mientras dure la fase. Se basa en D45, D55, D56 y D58 ([DECISIONS.md](../DECISIONS.md)) y en el [ROADMAP](../ROADMAP.md). Al cerrar la fase pasa a `historico/`. Escrito el 27/09/2026 sin el maintainer: todo lo que no está en esas decisiones es **propuesta** y está marcado así.

**Hecho cuando:** mi hijo ve la biblioteca familiar y pide añadir un libro; yo lo apruebo desde el portátil; mi pareja edita sin pedir nada.

## Hitos

Cada hito deja la app funcionando, con sus tests, y se prueba antes de seguir.

| Hito | Qué | Dónde |
|---|---|---|
| **5.1 Firmas** (D56) | Cada dispositivo tiene su clave Ed25519, firma sus operaciones y los demás comprueban la firma. La clave pública de cada dispositivo viaja con la biblioteca | `spec/`, `crates/domain`, `ops.rs`, `sync.rs`, `server/` |
| **5.2 Personas y dispositivos** | Cada dispositivo de una biblioteca es de una persona (un miembro): «el móvil de Hugo» es de Hugo. Se dice al compartir | `spec/`, `sync.rs`, `lan/` |
| **5.3 Roles** | Administrador, editor y lector, y el permiso de compartir aparte. Nadie da más de lo que tiene; puede haber varios administradores | `spec/`, `crates/domain` |
| **5.4 Comprobar lo que llega** | Cada dispositivo comprueba cada operación contra los roles que tenía su autor en ese momento. Todos llegan al mismo resultado aunque las cosas lleguen en otro orden | `sync.rs`, casos compartidos en `spec/` |
| **5.5 Peticiones** | Lo que un rol no puede hacer se convierte en petición; los administradores aceptan o rechazan | `spec/`, `app/` |
| **5.6 Dispositivo perdido** | Un administrador quita un dispositivo perdido desde otro y vincula el nuevo (D58). Aviso cuando hay un solo administrador con un solo dispositivo. Último recurso: duplicar la biblioteca como nueva | `libraries.rs`, `lan/` |
| **5.7 Interfaz** | Personas y permisos, peticiones, «Pedir que se borre» y a quién pertenece cada dispositivo | `app/src`, encargo a Claude Design |
| **5.8 Marcas por persona** (D68) | Cada persona marca en cada obra su estado (pendiente, en curso, terminado o abandonado, con fecha), su valoración y si es favorita. Cada una marca lo suyo, sea cual sea su rol | `spec/`, `crates/domain`, `app/`, encargo a Claude Design |

Las pantallas de 5.2, 5.5 y 5.7 se piden a Claude Design antes de construirlas. Encargo revisado el 27/09/2026: [design/encargo-fase-5.md](../design/encargo-fase-5.md).

## Decisiones técnicas propuestas

Detalles de construcción que no cambian lo decidido. Son **propuestas** hasta que el maintainer las revise.

### 5.1 Firmas

- **Qué se firma:** la operación entera (`id`, `library`, `hlc`, `intent`, `changes`) en **forma canónica**: JSON con las claves ordenadas, sin espacios y con los textos tal cual (RFC 8785, JCS). Rust y PHP tienen que producir exactamente los mismos bytes; `spec/` lleva ejemplos con los bytes y la firma esperados para comprobarlo.
- **Quién firma:** el dispositivo que aparece en el `hlc` de la operación. La firma va en un campo nuevo, `signature` (Ed25519, base64url sin relleno).
- **Cómo llega la clave pública a los demás:** una entidad nueva de la biblioteca, `device`, con su clave pública y su nombre. El dispositivo la crea él mismo la primera vez que escribe en una biblioteca, en la misma operación, firmada con esa clave: así demuestra que la tiene. Como viaja con la biblioteca, se puede comprobar una operación que llega a través de otro dispositivo.
- **La primera clave de un dispositivo es la que vale.** Una operación firmada con otra clave para el mismo dispositivo se rechaza.
- **Al recibir:** firma que no cuadra, o de un dispositivo sin clave conocida → la operación se rechaza y se cuenta, como las de otra versión. El orden por HLC garantiza que la clave de un dispositivo llega antes que sus operaciones firmadas.
- **La clave privada** vive en la base de datos del dispositivo (`local_state`, `signing_seed`), no en un fichero aparte como el certificado de la red local: así la tiene a mano cada escritura sin más plumbing, y viaja con la base de datos cuando se copia la carpeta de datos (D70). No va en las copias de seguridad (D58). *Cambiado al construirlo.*
- **La clave pública de un dispositivo no cambia nunca:** una operación que intente poner otra en su fila `device` se rechaza. *Añadido al construirlo.*
- **Operaciones sin firma** (las de antes de esta fase): se siguen aceptando mientras la biblioteca no tenga roles; ver la pregunta 1.

### 5.3 y 5.4 Roles y comprobación

- **El rol es un campo del miembro** (`role`: `admin`, `editor` o `reader`; `can_share`: sí o no), no una entidad aparte. Cambiarlo es una operación más.
- **Una biblioteca sin roles asignados funciona como hoy:** todos pueden todo. Los roles se activan con la primera asignación. Así nada cambia para quien no los use.
- **Converger con permisos:** el estado de una biblioteca es el resultado de aplicar su registro **en orden de HLC**, comprobando cada operación contra los roles vigentes en ese momento. Si llega tarde una operación que cambia permisos (un administrador quitó el rol de editor mientras el editor, sin conexión, seguía cambiando cosas), el dispositivo **recalcula esa biblioteca desde su registro**. Recalcular es posible porque el registro se guarda entero, y en una biblioteca familiar son miles de operaciones, no millones. Así todos llegan al mismo resultado sea cual sea el orden de llegada.
- **Lo que no está permitido no se pierde:** la operación se queda en el registro y se convierte en petición (5.5).

### 5.5 Peticiones

- **Una petición apunta a la operación que no se permitió** y no crea nada por sí misma. Aceptarla es una operación nueva del administrador que aplica esos cambios; rechazarla, otra que la cierra. Así quien acepta firma lo que entra.

### 5.8 Marcas por persona

- **Una fila `mark` por persona y obra**, con `member_id`, `work_id`, `status` (`pending`, `in_progress`, `finished` o `abandoned`), `status_on` (fecha), `rating` y `favorite`. Todo opcional salvo las dos referencias.
- **Su id se deriva de la persona y la obra** (UUID v5 de `member_id` y `work_id`), en vez de ser aleatorio: si la misma persona marca la misma obra en dos dispositivos sin conexión, las dos marcas son la misma fila y se fusionan campo a campo. Rust y PHP calculan el mismo id; `spec/` lo fija con ejemplos.
- **Regla de permisos:** cualquiera con persona, sea cual sea su rol, crea, cambia y borra las marcas de **su** persona; quien administra, las de todas. Un dispositivo sin persona o una operación sin firma, ninguna. Va en `spec/README.md` con sus casos de conformidad.
- **Valoración de 1 a 5**, sin medias estrellas.
- **La fecha** se pone sola al cambiar el estado (hoy) y se puede corregir.
- **Borrar una obra borra sus marcas**, en la misma operación.
- **Interfaz:** en la ficha, «Lo mío» con estado, fecha, estrellas y favorito; en el catálogo, un filtro por mi estado y por favoritos. Pantallas a Claude Design antes de construirla.

## Pendiente de decidir

Las preguntas 1 y 2 y las reglas del borrador se aceptaron el 27/09/2026: [D73](../DECISIONS.md). Quedan:

1. ~~Las marcas por persona (D68): ¿en esta fase?~~ Sí: hito 5.8 (27/09/2026).

## Estado

- [x] Plan escrito (27/09/2026).
- [x] Borrador de casos de roles y permisos: `spec/conformance/draft/permissions.json` (10 casos) y sus reglas en `spec/conformance/draft/README.md`. **Para revisar** antes de construir 5.3 y 5.4.
- [x] 5.1 Firmas (27/09/2026): especificación con casos de firma compartidos (`spec/conformance/signatures.json`), entidad `device`, regla `other_device`, firma y comprobación en Rust, comprobación en PHP con libsodium, migración 11. La app firma todo lo que escribe y rechaza lo que no cuadra; tests con tres dispositivos, manipulación en el camino, suplantación y cambio de clave. El APK compila. Pruebas en pantalla: [`docs/testing/fase-5.md`](../testing/fase-5.md).
- [x] 5.2 Personas y dispositivos, los datos (27/09/2026): `member_id` en la fila `device`. Decir de quién es cada dispositivo al emparejar llega con la interfaz (5.7).
- [x] 5.3 Roles, los datos (27/09/2026): `role` y `can_share` en el miembro, con un tipo de campo booleano nuevo en la especificación, Rust y PHP. Migración 12.
- [x] 5.4 Comprobar lo que llega (27/09/2026): reglas en `crates/domain/src/permission.rs`; la app juzga lo que recibe y lo que escribe, guarda lo no permitido como petición y recalcula la biblioteca desde su registro cuando llega tarde algo con roles. Los 10 casos pasan de borrador a `spec/conformance/permissions.json`. **Corrección de los casos:** sus órdenes de llegada «al revés» desordenaban las operaciones de un mismo dispositivo, algo que la sincronización nunca hace (y con firmas, una operación llegaría antes que la clave de su autor). Se cambiaron por órdenes que respetan el de cada dispositivo; las reglas y los resultados esperados no cambian. También se corrigió un id que no era UUID.
- [x] 5.5 Peticiones, sin interfaz (27/09/2026): modo petición para escribir lo que el rol no permite (`ops::as_request`), entidad `change_request` (migración 13), listar, aceptar y rechazar (`requests.rs` y sus comandos). Tests de extremo a extremo: un lector pide un libro en su móvil y el administrador lo acepta desde el portátil; y el caso de conformidad «solo un administrador resuelve una petición». Pendiente para después: que quien pidió pueda retirar su petición.
- [x] 5.6 Dispositivo perdido, sin interfaz (27/09/2026): `people.rs` con listar personas (y el aviso de administrador único con un solo dispositivo), activar los roles, cambiar roles (el último administrador no puede dejar de serlo), vincular dispositivos y quitar un dispositivo perdido de todas sus bibliotecas; `backup::duplicate` para duplicar una biblioteca como nueva. Restaurar y duplicar no copian los roles.
- [x] 5.7 Interfaz (27/09/2026). Por partes, un commit cada una, para poder dejarlo a medias sin romper nada. Pantallas: `docs/design/screens/{desktop,mobile}-people/`.
  - [x] **a. Peticiones, lo que falta del motor** (27/09/2026): nota de quien pide, retirar una petición propia, quién la resolvió y cuándo, y aviso de «un cambio tuyo ha pasado a petición». `change_request` admite `note` y el estado `withdrawn` (migración 14, esquema, PHP y casos de conformidad, incluido uno de permisos: solo retira quien pidió).
  - [x] **b. Personas, lo que falta del motor** (27/09/2026): aviso con una sola persona administradora (D58), activar los roles diciendo de quién es cada dispositivo (editor por defecto), la misma persona en todas las bibliotecas (`known_people`) y dar acceso al emparejar con el rol elegido (`give_access`; sin rol elegido no entra en una biblioteca). Migración 15.
  - [x] **c. Personas y permisos (pantalla)** (27/09/2026): tabla en escritorio y lista con ficha en móvil, menú de rol (bloqueado para la única persona administradora), puede compartir, aviso de una sola persona administradora con «Nombrar a otra persona», dispositivos sin persona con «Asignar», vista de solo lectura, activar los permisos (explicación y paso «¿De quién es cada dispositivo?», con persona nueva y editor por defecto) y quitar un dispositivo perdido. Los errores de permisos llegan a la interfaz con su propio tipo.
  - [x] **d. Peticiones (pantallas)** (27/09/2026): bandeja de quien administra con pendientes y resueltas (dos paneles en escritorio, tarjetas en móvil), «Qué cambiaría si aceptas» con la nota de quien pide, aceptar, rechazar y editar y aceptar (solo para libros nuevos, con el formulario de añadir), avisos de «ya la resolvió otra persona» y «ya no se puede aplicar», «Mis peticiones» con estado y retirar, campanita con el número en las barras superiores y contador en el menú. En móvil, personas y peticiones llevan su propia cabecera con «Atrás», como en Design, sin la barra superior.
  - [x] **e. Pedir en lugar de hacer** (27/09/2026): quien lee ve «Pedir que se añada» en la barra superior, el botón flotante del catálogo y ¿Lo tengo?; el formulario de añadir se convierte en petición con nota (sin descargar la portada, que se quedaría en su dispositivo). En la ficha, «Editar» y «Añadir ejemplar» desaparecen para quien lee, y «Borrar» pasa a «Pedir que se borre» (con nota) en lo que ese dispositivo no puede borrar: la ficha lo sabe juzgando en seco los mismos cambios con las mismas reglas (`ops::would_allow`). Al sincronizar, si un cambio propio ha pasado a petición, sale el aviso con «Ver mi petición». Quien lee también apunta deseos (añadido a D73): una regla nueva en la especificación, con su caso de conformidad.
  - [x] **f. Dispositivos y emparejar** (27/09/2026): columna «De quién es» en dispositivos (en móvil, «Es de»), que quien administra cambia o asigna y los demás solo ven. Tras emparejar, si el dispositivo recibe alguna biblioteca con roles que administras, un paso más: «¿De quién es?» con las personas de esas bibliotecas u «Otra persona», y el rol donde aún no lo tenga (nunca se hereda); en móvil, los roles van en un tercer paso solo si faltan. «Ahora no» lo deja sin persona (cuenta como lector). `give_access` ya no toca bibliotecas sin roles ni las que no administras. **Diferencias con Design:** no hay columna «Rol» en dispositivos (está en Personas y permisos) ni se detecta «Sustituye al Móvil de Hugo»; «Puede compartir» se comprueba al compartir (añadido a D73): quien no puede no ve la opción, con el aviso de Design, y el motor lo rechaza.
  - [x] **g. Documentación** (27/09/2026): lista de pruebas en pantalla en `docs/testing/fase-5.md`. La fase se cierra cuando pasen.
- [ ] 5.8 Marcas por persona (D68): datos y regla, casos de conformidad, pantallas de Design e interfaz.

### 5.7 Decisiones técnicas propuestas

Detalles de construcción de lo aceptado en D73. **Propuestas** hasta que el maintainer las revise.

- **La nota de una petición viaja dentro de ella:** al pedir, la operación lleva además una fila `change_request` con su propio id y el campo `note`. Esa fila solo puede crearla quien administra, así que la operación sigue siendo una petición y nada se aplica; la nota queda en el registro con lo pedido. Al aceptar, se repiten los cambios pedidos salvo esa fila. `status` pasa a ser opcional y admite `withdrawn`.
- **Retirar:** quien pidió (o otro dispositivo de su misma persona) crea la fila `change_request` con `status: withdrawn`. Es la única excepción a «las peticiones las resuelve quien administra», con su caso de conformidad.
- **Quién resolvió y cuándo:** sale del reloj (HLC) con que se escribió el estado de la petición, que dice el dispositivo y la hora. No hace falta guardarlo aparte.
- **«Un cambio tuyo ha pasado a petición»:** cuando al recalcular una biblioteca una operación de este dispositivo pasa de aplicada a petición, se apunta en una tabla local para avisar una vez.
- **Un dispositivo recién emparejado se registra al recibir la biblioteca** (por la red local), aunque no escriba nada: solo él puede crear su fila `device`, y sin ella nadie puede decir de quién es.
- **Asignación pendiente:** si quien administra dice de quién es un dispositivo que aún no se ha registrado en la biblioteca, se guarda en local (`pending_owners`) y se registra en cuanto llega su registro, en la misma sincronización.
- **«Editar y aceptar»:** para peticiones de añadir un libro, abre el formulario con lo pedido y acepta con lo corregido en una sola operación. Para las demás peticiones (borrar, cambiar), solo aceptar o rechazar.
