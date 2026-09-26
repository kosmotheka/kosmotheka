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

Las pantallas nuevas del hito 4.5 (nueva biblioteca, elegir bibliotecas al emparejar, borrar, mover) se piden a Claude Design antes de construirlas.

## Decisiones técnicas propuestas

Detalles de construcción que no cambian lo decidido; se apuntan aquí y se revisan al llegar a cada hito.

- **Sin clave foránea hacia `libraries`.** Un cambio se aplica aunque la fila de la biblioteca no haya llegado todavía (caso «changes apply before the library itself arrives»). La biblioteca es el ámbito, no un padre que tenga que existir antes.
- **Borrar la biblioteca deja una lápida de biblioteca**: se borran sus filas y cualquier cambio posterior en ella se ignora.
- **Operaciones con su biblioteca en el registro** (`operations.library_id`), para calcular el vector y lo que falta a cada dispositivo por biblioteca.
- **Qué comparte cada dispositivo:** una tabla local `library_peers (library_id, device_id)`. Solo se envían y se aceptan operaciones de bibliotecas compartidas con ese dispositivo.
- **Copia automática antes de migrar.** La migración 4.3 toca todas las tablas: antes de aplicarla, la app guarda una copia del fichero de la base de datos junto a él (`library.db.antes-fase-4`). Si algo sale mal, basta con cerrar la app y poner esa copia en su sitio. Se documenta en la guía.

## Pendiente de decidir

- **Continuidad al migrar.** D46 dice «migración sin continuidad»: cada dispositivo pasa sus datos a una biblioteca con un id propio, y hay que volver a emparejar asumiendo duplicados. Hay una alternativa: dar a la biblioteca migrada **el mismo id en todos los dispositivos**. Los que ya estaban sincronizados tienen las mismas filas con los mismos ids, así que al volver a emparejar se funden sin duplicados. El único riesgo: si dos casas que nunca se sincronizaron se emparejan y comparten esa biblioteca, sus catálogos se juntan en uno. Hoy solo la usas tú.
- **Mostrar quién creó cada biblioteca** (D65): sacarlo de la operación que la creó.

## Estado

- [x] 4.1 Casos de validación del formato: `spec/conformance/draft/library-scope.json` (13 casos, revisados).
- [x] 4.1 Casos de fusión por biblioteca: `spec/conformance/draft/library-merge.json` (8 casos) y reglas en `spec/README.md`.
- [ ] 4.2 Dominio en Rust y PHP.
- [ ] 4.3 Base de datos y migración.
- [ ] 4.4 Sincronización por biblioteca.
- [ ] 4.5 Interfaz.
- [ ] 4.6 Copias.
