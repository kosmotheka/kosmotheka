# Plan detallado de la Fase 3: servidor y familia

> **Borrador para revisar.** Desarrolla los hitos de la Fase 3 del [ROADMAP](../ROADMAP.md#fase-3--servidor-y-familia). Las decisiones que ya están tomadas se citan (D38, D39); las que propongo aquí están marcadas con **❓** y esperan tu visto bueno. Cuando se acepten, pasan a [DECISIONS.md](../DECISIONS.md).

## Estado

| Hito | Estado |
|---|---|
| 3.0 Esqueleto del servidor | ✅ Hecho (commit `7b452c5`): Docker, Symfony 8.1, PostgreSQL 18, validación de operaciones en PHP con los 22 casos de `spec/`, `make check` y trabajo en la CI |
| 3.1 Aplicar operaciones en PHP | Siguiente |
| 3.2 Cuentas, bibliotecas, roles e invitaciones | — |
| 3.3 La app habla con el servidor | — |
| 3.4 Solicitudes y despliegue | — |

## Arquitectura del monorepo (confirmada)

```
app/            Tauri + React. Compila en el contenedor Distrobox library-dev (fuera del repo)
crates/domain/  Dominio en Rust
spec/           Formato de operaciones y casos de conformidad (MIT). Lo único compartido
server/         Symfony. Su propio Docker Compose en server/docker/ (proyecto "library")
docs/
```

- Los Docker no se mezclan: solo `server/docker/` define contenedores, con nombres `*_library`, subred `172.194.0.0/24` y puertos 8095 (HTTP) y 5433 (PostgreSQL), elegidos para no chocar con Riddle ni rpgutilities.
- Las recetas de Symfony Flex tienen desactivada la generación de `compose.yaml` (`extra.symfony.docker: false`), así que no aparecerá otro Docker en la raíz de `server/`.
- `spec/` se monta en solo lectura dentro del contenedor PHP.

## 3.1 Aplicar operaciones en PHP

**Objetivo:** el servidor aplica operaciones con las mismas reglas de fusión que la app (D25, D32), y lo demuestran casos compartidos.

1. **Casos de conformidad de la fusión** en `spec/conformance/merge.json`: cada caso es un estado inicial, una lista de operaciones que llegan en cierto orden y el estado final esperado (filas, campos y lápidas). Primero se escriben y se hacen pasar en **Rust**, contra el `sync::merge` actual, y después en PHP. Cubren: el último gana campo a campo por HLC, el borrado gana, una operación repetida no cambia nada, el orden de llegada no importa, y el cambio imposible (por ejemplo, una referencia a una fila borrada) se salta sin tumbar la operación entera.
2. **Tablas del catálogo en PostgreSQL**, espejo de las de SQLite, con `library_id` en todas (D38).
   - ❓ **Clave primaria `(library_id, id)`** en vez de solo `id`. Los UUID v7 no deberían repetirse, pero así una biblioteca nunca puede pisar filas de otra, aunque un cliente mande un id ajeno. Recomendado.
   - Tablas de soporte: `operations` (el registro, con el cuerpo en JSONB: es un registro, no el modelo de datos), `field_versions` y `tombstones`, igual que en el cliente.
3. **Motor de aplicación con DBAL** en `server/src/Sync/`, sin ORM: SQL directo en transacción, con *savepoints*, como en Rust. El ORM se reserva para usuarios y bibliotecas (3.2).
4. **Hecho cuando:** los casos de `merge.json` pasan en Rust y en PHP, en la CI.

## 3.2 Cuentas, bibliotecas, roles e invitaciones

**Objetivo:** la API sabe quién eres, en qué bibliotecas estás y qué puedes hacer en cada una.

- **Entidades (ORM):**

  | Entidad | Contenido |
  |---|---|
  | `User` | Email, contraseña con hash (argon2id, lo que da Symfony por defecto), nombre |
  | `Library` | Nombre, fecha de alta |
  | `Membership` | Usuario, biblioteca, rol (`admin`, `editor`, `reader`) |
  | `Device` | Usuario, nombre, hash del token, último acceso |
  | `Invitation` | Biblioteca, rol, hash del token, caducidad y uso único |

- **Autenticación (D38):** usuario y contraseña para dar de alta un dispositivo; a partir de ahí, el dispositivo usa su propio token (`Authorization: Bearer`). En la base de datos solo se guarda el hash del token, así que una copia de la base filtrada no sirve para entrar. Cada dispositivo se puede revocar por separado.
- ❓ **Sin registro abierto.** El primer administrador se crea por consola (`make console C="app:user:create"`); los demás entran por invitación. Un servicio público vendría después, sin tocar el modelo. Recomendado.
- **Invitación por QR:** contiene la URL del servidor y un token de un solo uso con caducidad (❓ 7 días). Al escanearlo, la app crea la cuenta (o inicia sesión) y se une a la biblioteca con el rol de la invitación.
- **Matriz de permisos** sobre los cambios, nunca sobre la etiqueta de intención (D25):

  | Rol | Puede |
  |---|---|
  | Lector | Nada en el catálogo; solo leer. ❓ ¿Puede apuntar deseos y (Fase 4) sus lecturas? Propongo que sí: son suyos |
  | Editor | Crear y editar todo; borrar solo lo que creó él. ❓ ¿O no borrar nunca y pedirlo con una solicitud? |
  | Administrador | Todo, más gestionar miembros, invitaciones y solicitudes |

- **API:** JSON bajo `/v1/`, con los mismos nombres de ruta que la sincronización por wifi cuando la semántica coincide.
- **Hecho cuando:** los tests funcionales cubren alta de dispositivo, invitación, revocación y rechazo por permisos, contra la base `library_test`.

## 3.3 La app habla con el servidor

**Objetivo:** «Conectar esta biblioteca a un servidor», y desde entonces sincronizar solo con él (D39).

- **Mismo protocolo que por wifi** (vector de versiones, `pull` y `push`; ver [sync.md](../architecture/sync.md)), pero sobre HTTPS con un certificado real (sin huella fijada) y con el token del dispositivo. La app reutiliza casi todo el código de sincronización.
- **Primera conexión:** la app sube todo su historial de operaciones; el servidor las valida, comprueba los permisos y las aplica. Si el usuario se une a una biblioteca que ya existe en el servidor, primero la descarga. ❓ ¿Qué pasa si se conecta una biblioteca local que ya tiene libros a una biblioteca del servidor que también tiene? Propongo no permitirlo en esta fase: o subes la tuya como biblioteca nueva o te unes a una existente empezando de cero en ese dispositivo.
- **Permisos en la app (D40):** el servidor dice el rol; la app oculta lo que ese rol no puede hacer.
- ❓ **Operaciones rechazadas.** Con D40 no deberían darse, pero pueden aparecer si te bajan el rol mientras estás sin conexión. Propongo que el servidor responda con el estado actual de las filas afectadas como una **operación de corrección** con su propio HLC, que llega a todos por la vía normal y deshace el cambio local. El borrado es la excepción: un borrado rechazado deja la fila borrada en ese dispositivo hasta que la reciba de nuevo del servidor, así que el servidor tiene que poder «resucitarla» solo en ese dispositivo. Es el punto más delicado de la fase; lo concretaré con casos de conformidad antes de programarlo.
- La sincronización por wifi se desactiva para una biblioteca conectada; los dispositivos emparejados se olvidan con un aviso.
- **Hecho cuando:** una prueba de extremo a extremo con el servidor en Docker y dos instancias de la app converge, y un lector no puede modificar el catálogo.

## 3.4 Solicitudes y despliegue

- **Solicitudes:** una operación que tu rol no permite se guarda como petición en el servidor (no se aplica ni se propaga). El administrador la ve, la aprueba (el servidor la aplica en su nombre, dejando constancia de quién la pidió) o la rechaza. En la app, «Pedir que se borre» sustituye a «Borrar» para quien no puede borrar.
- **Despliegue:** una imagen para producción. ❓ **FrankenPHP** (el modelo de Koillection: un solo contenedor con Caddy, que saca el certificado HTTPS solo) más PostgreSQL, en vez de nginx y PHP-FPM, que es lo que usamos en desarrollo. Para el VPS necesitaré: el dominio o subdominio, si ya hay un proxy delante (otro Caddy o nginx con más servicios) y cómo quieres hacer las copias de seguridad de PostgreSQL.
- **Hecho cuando:** la familia usa el servidor del VPS desde el escritorio y el móvil.

## Resumen de preguntas abiertas

1. Clave primaria `(library_id, id)` en las tablas del catálogo (recomendado).
2. Sin registro abierto: primer administrador por consola, el resto por invitación (recomendado).
3. Caducidad de las invitaciones (propuesta: 7 días, un solo uso).
4. Lector: ¿puede apuntar deseos y sus lecturas? (propuesta: sí).
5. Editor: ¿borra lo que creó o nunca borra? (propuesta: borra lo suyo).
6. Conectar una biblioteca con datos a otra del servidor que también tiene: no permitirlo por ahora (propuesta).
7. Operaciones rechazadas: operaciones de corrección del servidor (propuesta, se detallará con casos de prueba).
8. Producción con FrankenPHP; datos del VPS pendientes.
