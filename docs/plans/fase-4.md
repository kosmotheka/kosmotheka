# Plan de la Fase 4 — Bibliotecas

> Plan de trabajo mientras dure la fase. Se basa en D46, D54 y D65 ([DECISIONS.md](../DECISIONS.md)) y en el [ROADMAP](../ROADMAP.md). Al cerrar la fase pasa a `historico/`.

**Hecho cuando:** tengo en el portátil mi biblioteca de rol y la familiar, y el móvil de mi pareja recibe solo la familiar.

## Hitos

Cada hito deja la app funcionando, con sus tests, y se prueba antes de seguir.

| Hito | Qué | Dónde |
|---|---|---|
| **4.1 Spec** | El formato con biblioteca y las reglas de fusión por biblioteca, como casos de conformidad | `spec/` |
| **4.2 Dominio** | `Operation` con `library`, la entidad `library` y los códigos de error nuevos, en Rust y en PHP. Los casos pasan de `draft/` a `conformance/` | `crates/domain`, `server/` |
| **4.3 Base de datos** | Tabla `libraries`, `library_id` en todas las tablas con clave `(library_id, id)`, versiones de campo y lápidas por biblioteca, y la migración de los datos actuales | `app/src-tauri` |
| **4.4 Sincronización por biblioteca** | Vector de versiones por biblioteca; al emparejar se elige qué bibliotecas van a cada dispositivo; solo viaja lo compartido | `sync.rs`, `lan/` |
| **4.5 Interfaz** | Selector de verdad, nueva biblioteca, renombrar, borrar (con confirmación y copia, D65), salir, mover un libro, buscar «en todas» y «¿Lo tengo?» en todas | `app/src` |
| **4.6 Copias** | Copia completa del dispositivo y copia o CSV de una biblioteca suelta | `backup.rs`, `csv_io.rs` |

Las pantallas de los hitos 4.4 y 4.5 se piden a Claude Design antes de construirlas: [encargo](../design/encargo-fase-4.md), subido al proyecto de Design el 26/09/2026.

## Decisiones técnicas propuestas

Detalles de construcción que no cambian lo decidido; se apuntan aquí y se revisan al llegar a cada hito.

- **Sin clave foránea hacia `libraries`.** Un cambio se aplica aunque la fila de la biblioteca no haya llegado todavía (caso «changes apply before the library itself arrives»). La biblioteca es el ámbito, no un padre que tenga que existir antes.
- **Borrar la biblioteca deja una lápida de biblioteca**: se borran sus filas y cualquier cambio posterior en ella se ignora.
- **Operaciones con su biblioteca en el registro** (`operations.library_id`), para calcular el vector y lo que falta a cada dispositivo por biblioteca.
- **Qué comparte cada dispositivo:** una tabla local `library_peers (library_id, device_id)`. Solo se envían y se aceptan operaciones de bibliotecas compartidas con ese dispositivo.
- **Migración con continuidad** (D46, cambiado el 26/09/2026): la biblioteca migrada tiene el mismo id fijo en todos los dispositivos, así que los que ya se sincronizaban se funden sin duplicados al volver a emparejar.
- **Al borrar una biblioteca se conserva su operación de borrado** en el registro local (D66).
- **Sin nombres repetidos en un dispositivo** (propuesto por Design y aceptado el 26/09/2026): no se puede crear ni renombrar una biblioteca con el nombre de otra que ya está en este dispositivo; el aviso ofrece abrir la existente. Es una regla de la interfaz, no del formato: entre dispositivos no se puede garantizar, y dos bibliotecas con el mismo nombre que lleguen por sincronización se muestran tal cual.
- **Copia automática antes de migrar.** La migración 4.3 toca todas las tablas: antes de aplicarla, la app guarda una copia del fichero de la base de datos junto a él (`library.db.antes-fase-4`). Si algo sale mal, basta con cerrar la app y poner esa copia en su sitio. Se documenta en la guía.

## Pendiente de decidir

- **Mostrar quién creó cada biblioteca** (D65): sacarlo de la operación que la creó.

## Estado

- [x] 4.1 Casos de validación del formato: `spec/conformance/draft/library-scope.json` (13 casos, revisados).
- [x] 4.1 Casos de fusión por biblioteca: `spec/conformance/library-merge.json` (8 casos, con órdenes de entrega que respetan la causalidad) y reglas en `spec/README.md`.
- [x] 4.2 Dominio en Rust y PHP: `library` en la operación, entidad `library`, códigos `invalid_library` y `other_library`; los 35 casos de validación pasan en los dos. La app usa por ahora una biblioteca fija (`DEFAULT_LIBRARY_ID`), la que tendrá la migrada.
- [x] 4.3 Base de datos y migración (migración 9): `library_id` y clave `(library_id, id)` en todas las tablas, versiones y lápidas por biblioteca, borrar una biblioteca, copia automática antes de migrar, protocolo con un vector por biblioteca y `library_peers`. Los casos de fusión pasan en Rust.
- Pendiente de la 4.3, para más adelante: purgar del registro local el contenido de una biblioteca borrada (hoy se conserva entero; D66 solo exige la operación de borrado).
- [ ] 4.4 Sincronización por biblioteca.
- [x] 4.5 Interfaz, lo que no depende de otros dispositivos: selector con las bibliotecas reales (barra lateral y menú del móvil), nueva biblioteca (con la regla de nombre único), ajustes de la biblioteca (renombrar, con quién se comparte, quién la creó), borrar con aviso y copia previa, salir (si solo está en este dispositivo, lleva a borrar para no perderla), mover un libro, buscar «en esta» o «en todas» y «¿Lo tengo?» en todas. Pruebas en pantalla: [`docs/testing/fase-4.md`](../testing/fase-4.md).
- [ ] 4.5 Pendiente, va con la 4.4: elegir bibliotecas al emparejar y «Dejar de compartir».
- Decidido para la 4.4 (D71): dejar de compartir retira el acceso y borra la biblioteca en el otro dispositivo cuando vuelve a conectar; la lista de revocados viaja como operación de la biblioteca.
- [ ] 4.6 Copias.
