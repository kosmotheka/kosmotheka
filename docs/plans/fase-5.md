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
| **5.6 Dispositivo perdido** | Un administrador revoca un dispositivo perdido desde otro y da permisos al nuevo (D58). El caso del administrador único sigue abierto | `libraries.rs`, `lan/` |
| **5.7 Interfaz** | Personas y permisos, peticiones, «Pedir que se borre» y a quién pertenece cada dispositivo | `app/src`, encargo a Claude Design |

Las pantallas de 5.2, 5.5 y 5.7 se piden a Claude Design antes de construirlas; el encargo se prepara al llegar ahí.

## Decisiones técnicas propuestas

Detalles de construcción que no cambian lo decidido. Son **propuestas** hasta que el maintainer las revise.

### 5.1 Firmas

- **Qué se firma:** la operación entera (`id`, `library`, `hlc`, `intent`, `changes`) en **forma canónica**: JSON con las claves ordenadas, sin espacios y con los textos tal cual (RFC 8785, JCS). Rust y PHP tienen que producir exactamente los mismos bytes; `spec/` lleva ejemplos con los bytes y la firma esperados para comprobarlo.
- **Quién firma:** el dispositivo que aparece en el `hlc` de la operación. La firma va en un campo nuevo, `signature` (Ed25519, base64url sin relleno).
- **Cómo llega la clave pública a los demás:** una entidad nueva de la biblioteca, `device`, con su clave pública y su nombre. El dispositivo la crea él mismo la primera vez que escribe en una biblioteca, en la misma operación, firmada con esa clave: así demuestra que la tiene. Como viaja con la biblioteca, se puede comprobar una operación que llega a través de otro dispositivo.
- **La primera clave de un dispositivo es la que vale.** Una operación firmada con otra clave para el mismo dispositivo se rechaza.
- **Al recibir:** firma que no cuadra, o de un dispositivo sin clave conocida → la operación se rechaza y se cuenta, como las de otra versión. El orden por HLC garantiza que la clave de un dispositivo llega antes que sus operaciones firmadas.
- **La clave privada** vive en la carpeta de datos (`signing-key`, solo legible por el usuario), como el certificado de la red local. No va en las copias (D58).
- **Operaciones sin firma** (las de antes de esta fase): se siguen aceptando mientras la biblioteca no tenga roles; ver la pregunta 1.

### 5.3 y 5.4 Roles y comprobación

- **El rol es un campo del miembro** (`role`: `admin`, `editor` o `reader`; `can_share`: sí o no), no una entidad aparte. Cambiarlo es una operación más.
- **Una biblioteca sin roles asignados funciona como hoy:** todos pueden todo. Los roles se activan con la primera asignación. Así nada cambia para quien no los use.
- **Converger con permisos:** el estado de una biblioteca es el resultado de aplicar su registro **en orden de HLC**, comprobando cada operación contra los roles vigentes en ese momento. Si llega tarde una operación que cambia permisos (un administrador quitó el rol de editor mientras el editor, sin conexión, seguía cambiando cosas), el dispositivo **recalcula esa biblioteca desde su registro**. Recalcular es posible porque el registro se guarda entero, y en una biblioteca familiar son miles de operaciones, no millones. Así todos llegan al mismo resultado sea cual sea el orden de llegada.
- **Lo que no está permitido no se pierde:** la operación se queda en el registro y se convierte en petición (5.5).

### 5.5 Peticiones

- **Una petición apunta a la operación que no se permitió** y no crea nada por sí misma. Aceptarla es una operación nueva del administrador que aplica esos cambios; rechazarla, otra que la cierra. Así quien acepta firma lo que entra.

## Pendiente de decidir

Preguntas para el maintainer, en orden de urgencia:

1. **Operaciones sin firma.** Las de antes de la Fase 5 no llevan firma. Propuesta: se aceptan mientras la biblioteca no tenga roles; desde que tiene roles, una operación sin firma nueva se trata como de un lector (pasa a petición). ¿De acuerdo?
2. **¿Quién es administrador al estrenar los roles?** Propuesta: quien los activa, asignándose a sí mismo el primero. Hasta entonces, todos pueden todo.
3. **El administrador único que pierde su único dispositivo** (abierto en D58): código de recuperación, sugerir un segundo administrador al compartir, o duplicar la biblioteca.
4. **Las marcas por persona (D68: leído, valoración, favorito):** ¿entran en esta fase, ya que el lector «marca lo suyo», o van en la 9?

## Estado

- [x] Plan escrito (27/09/2026).
- [ ] 5.1 Firmas.
- [ ] 5.2 Personas y dispositivos.
- [ ] 5.3 Roles.
- [ ] 5.4 Comprobar lo que llega.
- [ ] 5.5 Peticiones.
- [ ] 5.6 Dispositivo perdido.
- [ ] 5.7 Interfaz.
