# Encargo · Los cuatro hallazgos que se arreglan antes de abrir el repositorio

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. El maintainer decidió el mismo día qué se arregla, qué se acepta y qué se descarta de los diez abiertos (`docs/audits/hallazgos.md`). Quedan cuatro por arreglar: CON-007, SEG-040, SEG-037 y SEG-074. **Empieza por la fase 0 y espera mi respuesta antes de escribir código.**

## Reglas de siempre que aquí pesan

- **Lo que decide quién recibe una biblioteca se prueba primero con el caso hostil, visto fallar** (AGENTS.md). Vale para SEG-040.
- **La sincronización tiene que converger**: un cambio en cómo se aplican las operaciones necesita su test en `sync.rs` con `assert_converged`, y una regla nueva va a `spec/` con casos que pasan Rust y PHP. Vale para CON-007.
- **Nada contra la base de datos de producción.** SEG-074 cambia datos cifrados: el código se escribe y se prueba aquí; pasarlo a los datos de verdad lo hace el maintainer con copia, al desplegar.
- Servidor: solo con `make` en Docker, y nunca dos `make check` a la vez (tarda unos 10 minutos).

## Los cuatro

### CON-007 · La app y el servidor tratan distinto una referencia a algo que nunca existió

Apuntado al comparar el motor de PHP con `sync::merge` (7.4.4). Si un cambio apunta a una fila que nunca existió, la app la aplica o se la salta según el orden de llegada; el servidor se la salta siempre. Los dos tienen que hacer lo mismo y no depender del orden.

- **Fase 0:** reprodúcelo con un test en `sync.rs` (dos órdenes de llegada, resultados distintos) y dime cuál es la regla buena. Lo esperado es la del servidor. Mira qué dicen `spec/README.md` y `docs/architecture/sync.md` sobre referencias rotas, y si D89 («el borrado gana») toca este caso.
- **Después:** el caso en `spec/conformance/` que pasen Rust y PHP, y el arreglo en la app.

### SEG-040 · Un servidor hostil reenvía una retirada antigua y calla la concesión posterior

`lan::remove_if_proved` (`app/src-tauri/src/lan/mod.rs`): un dispositivo que todavía no tiene la concesión posterior podría borrar una biblioteca por una retirada que ya se deshizo. Viene del informe del 10/10/2026 y está marcado como «posible».

- **Fase 0:** escribe el test hostil y dime si falla. Antes de proponer un arreglo, explícame qué puede saber de verdad el dispositivo. Si el servidor le calla operaciones posteriores, ¿hay algo en la cadena de D87 (`previous`) que le deje ver que falta algo? **Si no tiene arreglo sin cambiar el protocolo, dilo**: entonces lo llevo al maintainer para aceptarlo o decidir otra cosa, y no se inventa nada.

### SEG-037 · El permiso de avisos vale una hora tras el corte

`EventsToken::LIFETIME_SECONDS = 3600` (`server/src/Sync/EventsToken.php`). Quien pierde el acceso sigue oyendo durante una hora *cuándo* cambia la biblioteca, nunca *qué*.

- **Arreglo propuesto:** 10 minutos. La app ya lo renueva antes de que caduque (`a_token_near_its_expiry_is_renewed_before_it_runs_out`, en `cloud/events/tests.rs`).
- **Fase 0:** confirma que al renovar el servidor vuelve a mirar el acceso (si no, acortarlo no sirve), y cuántas peticiones de más hace la app por hora con 10 minutos. Mira también el límite `cloud_events` de `config/packages/rate_limiter.yaml`: que la renovación no lo agote.
- Test del servidor: tras el corte, la renovación no da permiso para esa biblioteca.

### SEG-074 · Las claves selladas de la nube no van ligadas a su biblioteca

`server/src/Security/CloudKeyVault.php:37`: `secretbox` sin datos asociados ni versión. Quien pueda escribir en la base podría copiar la clave de una biblioteca a otra.

- **Arreglo propuesto en la auditoría:** AEAD (por ejemplo `sodium_crypto_aead_xchacha20poly1305_ietf_*`) con el id de la biblioteca de la nube como dato asociado y un prefijo de versión en lo guardado.
- **Fase 0:** propón:
  - el formato nuevo;
  - cómo convive con las claves ya guardadas en el formato viejo: leer las dos y escribir solo la nueva, o convertirlas todas de una vez;
  - si la conversión es una migración de Doctrine o un comando de consola (la clave maestra está en el entorno, no en la base);
  - qué pasa si el despliegue se queda a medias.
- **El test hostil:** la clave de la biblioteca A copiada en la fila de B no se abre.
- **Documentación:** los pasos para el maintainer al desplegar, con la copia antes, en `docs/architecture/accounts.md` o donde viva hoy el cofre.

## Fase 0 · Lo que me mandas

Un informe con los cuatro puntos de arriba y, para cada uno, lo que tocarías. **Sin código todavía**, salvo los tests que reproducen CON-007 y SEG-040 (pueden quedarse en rojo).

## Al terminar (después de mi visto bueno)

- App: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run check:spec`.
- Servidor: `make check`, en segundo plano, una sola vez y al final.
- `docs/audits/hallazgos.md`: los que se cierren pasan a `resuelto`, con la fecha. Si alguno no se arregla, dímelo.
- Una línea en `app/src/whats-new.json` solo si una persona lo notaría. Ninguno de estos debería notarse; si alguno sí, el límite de cinco líneas de `next` ya está lleno: dímelo antes.
