# Modelo de amenazas

Escrito en la auditoría del 2026-10-03 (commit `18c5365`). Se revisa cuando cambian las decisiones de seguridad. Los hallazgos citados están en `hallazgos.md`.

## Activos

| Activo | Dónde vive | Sensibilidad |
|---|---|---|
| Catálogo (obras, ediciones, ejemplares, ubicaciones) | SQLite de cada dispositivo; PostgreSQL en la nube | Media: revela hábitos y patrimonio |
| Datos personales: miembros, préstamos, nombres de dispositivo, cuentas y emails | Filas `member`, `device`; cuentas en el servidor | Alta (RGPD) |
| Clave de firma Ed25519 (`signing_seed`) | `local_state` en SQLite (`ops.rs:20`) | Crítica: firma en nombre de la persona |
| Identidad TLS (`sync-key.der`) | Carpeta de datos, 0600 en Unix | Alta: suplantar al dispositivo en la LAN |
| Secretos de emparejamiento (de un uso y permanentes) | QR; `known_clients.secret_hash`, `known_servers` | Alta |
| Credenciales de cuenta y tokens | Servidor (hash); token por dispositivo (D09) | Alta |
| Copias de seguridad y exportaciones | Donde las ponga el usuario | Media-alta: sin claves (D58), con datos personales |
| Base de datos del servidor y registro de actividad (IP, un año, D66) | PostgreSQL | Alta |

## Atacantes

| Id | Qué tiene | Qué quiere |
|---|---|---|
| A1 | Lector con su dispositivo y una copia completa de la biblioteca | Modificar o borrar el catálogo, tocar marcas ajenas, compartir sin permiso |
| A2 | Editor | Hacerse administrador o `can_share`, borrar lo que no creó, vincular dispositivos |
| A3 | Antiguo miembro o dispositivo retirado con su copia y su clave | Seguir leyendo o escribiendo |
| A4 | Equipo en la misma LAN, nunca emparejado | Leer el tráfico, suplantar, emparejar con un QR visto de lejos, denegar el servicio |
| A5 | Móvil robado, bloqueado o desbloqueado | Leer datos, extraer claves, actuar como su dueño |
| A6 | Atacante de internet contra un servidor propio | Tomar cuentas, leer bibliotecas ajenas, abusar del correo |
| A7 | Proveedor de metadatos hostil o manipulado | Inyectar contenido, rastrear con imágenes, envenenar el catálogo |
| A8 | Dependencia maliciosa (crate, npm, composer, action) | Ejecutar código con acceso a claves y datos |
| A9 | Quien administra el servidor | Leer y editar cualquier biblioteca de la nube |

## Fronteras de confianza

1. Webview ↔ núcleo Rust (IPC de Tauri, capabilities `default.json` y `mobile.json`).
2. Dispositivo ↔ dispositivo (HTTPS en la LAN con certificado fijado: `/v1/pair`, `/v1/sync/*`, `/v1/hello`).
3. Dispositivo ↔ servidor (HTTPS, token por dispositivo, operaciones firmadas).
4. App ↔ proveedores de metadatos (HTTPS saliente, portadas).
5. Servidor ↔ base de datos (PostgreSQL en una red de Docker).
6. Repositorio ↔ CI (`.github/workflows/ci.yml`).

## STRIDE por frontera

**1 · Webview ↔ Rust**
- Suplantación y elevación: CSP `default-src 'self'`, `connect-src ipc:`, capabilities mínimas; toda escritura pasa por `ops::record`, que juzga los permisos.
- Fuga de información: `img-src https:` (SEG-006).
- Manipulación: el texto de los proveedores se muestra con React, sin `dangerouslySetInnerHTML` (triaje `ts.sink`). Pendiente el tema `webview-rendering`.

**2 · Dispositivo ↔ dispositivo**
- Suplantar al servidor: huella del certificado fijada por el QR (D34) e id de dispositivo comprobado en `/v1/hello` (D37).
- Suplantar al cliente: secreto permanente de 256 bits guardado como hash. Al emparejar, el `device_id` declarado no se comprueba (SEG-002).
- Falsificar al autor: firma Ed25519 con `verify_strict`; vale la primera clave registrada.
- Manipulación: TLS y la firma de cada operación, también reenviada.
- Repudio: registro de operaciones firmado; sin hora fiable en local (D66).
- Fuga: solo se envían bibliotecas compartidas por los dos lados (`library_peers`); `ensure_may_share` al abrir la ventana.
- Denegación: servidor que solo escucha cuando hace falta (D37); sin limitador en `/v1/pair`, innecesario con un secreto de un uso de 256 bits.
- Elevación: `permission::is_allowed`; sin firma cuenta como lector; nadie da más rol del que tiene; recálculo cuando llegan tarde cambios de rol (D73).

**3 · Dispositivo ↔ servidor**
- Suplantación: token por dispositivo (D09, D38), email verificado (D49), 2FA opcional (D78), recuperación que revoca los demás tokens (D50).
- Manipulación y elevación: la nube juzga con las mismas reglas y rechaza firmas falsas y dispositivos retirados (D73).
- Repudio: registro de actividad con hora e IP (D66).
- Fuga: el superadministrador lo ve todo (D51, aceptado).
- Denegación: limitadores por IP con `TRUSTED_PROXIES`. Pendiente el tema `server`.

**4 · App ↔ proveedores**
- Los proveedores ayudan y nunca son imprescindibles (D62). Las portadas se descargan, se validan y se guardan por hash (D27). Las claves de API son del usuario o van en el servidor (D83).

**5 · Servidor ↔ base de datos**
- Red interna de Docker y credenciales en `.env.local` sin versionar. Pendiente el tema `server`.

**6 · Repositorio ↔ CI**
- Actions de terceros sin fijar por SHA y sin `permissions:` (SEG-003). Sin `cargo audit` ni `composer audit` en el CI; la auditoría pasa osv-scanner.

## Por atacante

- **A1:** no escribe el catálogo (pasa a solicitud) ni toca marcas ajenas (el id de la marca se deriva de persona y obra). Lee todo, por diseño.
- **A2:** no toca `role` ni `can_share`, ni `Device`, `Library` o `Revocation`. Borra solo lo que creó su persona.
- **A3:** retirada con `library.revoke`; se rechaza lo que firme después (D71, D73). Conserva lo que ya tenía (aceptado).
- **A4:** TLS con huella fijada. Riesgo si ve el QR (SEG-002).
- **A5:** desbloqueado usa la app con el rol de su dueño; se corta con `device.remove` desde otro dispositivo (D58) y el bloqueo de la app. Bloqueado depende del cifrado del sistema; la copia de Android, SEG-001.
- **A6, A9:** ver frontera 3 y D51.
- **A7:** ver frontera 4 y SEG-006.
- **A8:** ver frontera 6 y SEG-003.

## Acceso a las bibliotecas

Revisado en la auditoría del 2026-10-03 (2), tema `library-access`. Quién puede acabar con los datos de una biblioteca.

**Garantías comprobadas** (cada una con el código que la asegura, en el informe):
- Por la red local, una biblioteca solo sale hacia un dispositivo si alguien la marcó para él (`library_peers`); lo que no está compartido no se pide, no se envía ni se acepta.
- Una retirada de acceso (D71) gana sobre la lista local de cada dispositivo.
- Compartir exige el permiso «Puede compartir» o ser administrador en el propio dispositivo.
- El código de emparejamiento es de un solo uso, caduca a los 10 minutos y no deja ocupar el sitio de un dispositivo ya emparejado.
- Un dispositivo dado a otra persona (D81) empieza con identidad nueva: nadie le reconoce con la antigua.
- Un dispositivo retirado no escribe en ningún sitio después de la retirada, aunque lo suyo llegue a través de otro que aún no lo sabe (SEG-015).
- En la nube, un dispositivo es su id junto con su clave: conocer el id de un móvil ajeno no permite ocuparlo (SEG-016).
- La app funciona entera sin servidor ni cuenta; nada de la nube planificada bloquea la red local.

**Límites conocidos:**
- «Mi biblioteca» tiene el mismo id en todas las instalaciones (D46). Por la red local, fundir dos catálogos ajenos exige que los dos dispositivos la marquen, salvo SEG-010 mientras siga abierto.
- Un compañero de mala fe ya emparejado puede hacer borrar una biblioteca en otro dispositivo (SEG-011, abierto).
- Quien pierde poder (rol, «Puede compartir», un dispositivo o el acceso) no puede antedatar cambios a antes de perderlo: cada operación que quita poder lleva una marca por dispositivo, y lo firmado entre la marca y la bajada pasa a petición (D84, SEG-014). Coste: una edición honrada hecha sin conexión justo antes de enterarse también pasa a petición.
- Lo que un dispositivo ya recibió no se puede borrar a distancia, y quien lo recibió a través de él lo conserva (D71).
- La nube (Fase 7) aún no existe: su control de acceso está pendiente de SEG-012 y SEG-013, y el tema vuelve a auditarse al terminar los hitos 7.2, 7.5 y 7.6.

## Riesgos aceptados

- Quien ya sacó datos (exportaciones, capturas, app modificada que ignora la retirada) los conserva (D71).
- La retirada llega cuando el afectado vuelve a conectar, y no alcanza a quien recibió la biblioteca a través de él (D71).
- Sin roles, cualquier dispositivo con la biblioteca puede borrarla o retirar a otro, también a quien la creó (D65, D71).
- Una app modificada puede compartir sin `can_share`; quien recibe cuenta como lector sin persona (D73).
- Se confía en el HLC de cada dispositivo: un reloj manipulado fecha mal sus cambios (D55, D73).
- El superadministrador tiene acceso total a la nube (D51).
- En local no hay hora fiable ni IP en el registro (D66).
- El administrador único que pierde su único dispositivo pierde el control; duplicar la biblioteca es el último recurso (D58; ver CON-002).
- Mientras la biblioteca no tenga roles se aceptan operaciones sin firma (D73).
- La nube entiende los datos, no es un relé cifrado; la privacidad total es el modo sin cuenta (D57).
