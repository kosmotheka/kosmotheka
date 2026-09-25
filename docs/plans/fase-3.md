# Plan detallado de la Fase 3: servidor y familia

> **Revisado el 25/09/2026** (D41–D52). Quedan abiertas **D46** (varias bibliotecas en la app) y **D52** (panel de administración), que hay que pensar antes de los hitos que dependen de ellas. Desarrolla los hitos de la Fase 3 del [ROADMAP](../ROADMAP.md#fase-3--servidor-y-familia). Las decisiones están en [DECISIONS.md](../DECISIONS.md); D40 sigue en `propuesta`.

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
   - ✅ **Clave primaria `(library_id, id)`** (D41), también en las claves foráneas: una biblioteca nunca puede pisar ni referenciar filas de otra.
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

- **Registro, verificación del email y recuperación de la contraseña (D49, D50):** el patrón de rpgutilities, con páginas web (Twig y formularios), enlace con token por correo y `UserChecker`. El proveedor de correo está por decidir (quizá Cloudflare).
- **Autenticación (D38):** usuario y contraseña para dar de alta un dispositivo; a partir de ahí, el dispositivo usa su propio token (`Authorization: Bearer`). En la base de datos solo se guarda el hash del token, así que una copia de la base filtrada no sirve para entrar. Cada dispositivo se puede revocar por separado.
- ✅ **Registro abierto** (D42): cualquiera crea su cuenta y sus bibliotecas; para unirse a una ajena, invitación. Con límite de peticiones por IP en el alta y el inicio de sesión.
- **Invitación por QR:** contiene la URL del servidor y un token de un solo uso que caduca en 24 horas (D43). Al escanearlo, la app crea la cuenta (o inicia sesión) y se une a la biblioteca con el rol de la invitación.
- **Matriz de permisos** sobre los cambios, nunca sobre la etiqueta de intención (D25):

  | Rol | Puede |
  |---|---|
  | Lector | Nada en el catálogo. Sí sus propios deseos y (Fase 4) sus lecturas (D44) |
  | Editor | Crear y editar todo; borrar solo lo que creó él (D45). Lo ajeno, con una solicitud |
  | Administrador | Todo, más gestionar miembros, invitaciones y solicitudes |

- **API:** JSON bajo `/v1/`, con los mismos nombres de ruta que la sincronización por wifi cuando la semántica coincide.
- **Hecho cuando:** los tests funcionales cubren alta de dispositivo, invitación, revocación y rechazo por permisos, contra la base `library_test`.

## 3.3 La app habla con el servidor

**Objetivo:** «Conectar esta biblioteca a un servidor», y desde entonces sincronizar solo con él (D39).

- **Mismo protocolo que por wifi** (vector de versiones, `pull` y `push`; ver [sync.md](../architecture/sync.md)), pero sobre HTTPS con un certificado real (sin huella fijada) y con el token del dispositivo. La app reutiliza casi todo el código de sincronización.
- **Primera conexión:** la app sube todo su historial de operaciones; el servidor las valida, comprueba los permisos y las aplica. Si el usuario se une a una biblioteca que ya existe en el servidor, primero la descarga. **Varias bibliotecas en la app (D46, abierta):** una entidad biblioteca en una sola base de datos, para estar a la vez en la de la familia y en otra con amigos; al unirse teniendo libros propios, la local se conserva y se pueden mover ítems. Hay que cerrar su arquitectura antes de este hito.
- **Permisos en la app (D40):** el servidor dice el rol; la app oculta lo que ese rol no puede hacer.
- ✅ **Operaciones rechazadas (D47):** el servidor responde a ese dispositivo con el estado válido de las filas afectadas y la app lo aplica encima; si era un borrado, levanta la lápida local y restaura la fila. La operación queda marcada como rechazada y la app avisa del motivo. Antes de programarlo, casos de conformidad en `spec/`.
- La sincronización por wifi se desactiva para una biblioteca conectada; los dispositivos emparejados se olvidan con un aviso.
- **Hecho cuando:** una prueba de extremo a extremo con el servidor en Docker y dos instancias de la app converge, y un lector no puede modificar el catálogo.

## 3.4 Solicitudes y despliegue

- **Solicitudes:** una operación que tu rol no permite se guarda como petición en el servidor (no se aplica ni se propaga). El administrador la ve, la aprueba (el servidor la aplica en su nombre, dejando constancia de quién la pidió) o la rechaza. En la app, «Pedir que se borre» sustituye a «Borrar» para quien no puede borrar.
- **Despliegue (D48):** nginx + PHP-FPM + PostgreSQL, como en desarrollo, con imágenes de producción (sin Xdebug ni dependencias de desarrollo) y un proxy con HTTPS delante. Para el VPS necesitaré: el dominio o subdominio, si ya hay un proxy delante (nginx, Caddy…) con más servicios y cómo quieres hacer las copias de seguridad de PostgreSQL.
- **Hecho cuando:** la familia usa el servidor del VPS desde el escritorio y el móvil.

## Preguntas

1. ~~Clave primaria `(library_id, id)`~~ → aceptada (D41).
2. ~~Registro~~ → abierto (D42). De ahí salen tres preguntas nuevas, al final de la lista.
3. ~~Caducidad de las invitaciones~~ → 24 horas, un solo uso (D43).
4. ~~Lector~~ → escribe solo sus deseos y lecturas (D44).
5. ~~Editor~~ → borra solo lo que creó (D45).
6. **Varias bibliotecas en la app** → entidad biblioteca en una sola base; arquitectura por cerrar (D46, abierta).
7. ~~Operaciones rechazadas~~ → corrección del servidor solo en ese dispositivo (D47).
8. ~~Producción~~ → nginx + PHP-FPM, como en desarrollo (D48). Datos del VPS: se piden al llegar al 3.4.
9. ~~Verificar el email~~ → obligatorio, patrón de rpgutilities; proveedor de correo por decidir (D49).
10. ~~Recuperar la contraseña~~ → patrón de rpgutilities; revoca los demás dispositivos (D50).
11. ~~Administrador global~~ → `ROLE_SUPER_ADMIN`, con acceso a todo (D51).
12. **Panel de administración** → EasyAdmin con la gestión típica y KPI; por diseñar (D52, abierta).
