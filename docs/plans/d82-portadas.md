# Plan: portadas que viajan entre dispositivos (D82)

> 24/10/2026. Decisión en [D82](../DECISIONS.md) (aceptada ese día). Va **antes de la próxima versión**: la prueba cerrada no debe encontrarse con obras sin portada en un dispositivo y con ella en otro.

**Estado (24/10/2026): las cinco partes hechas en código, con sus tests; falta la prueba en dispositivos reales.**

| Parte | Commit | Qué quedó |
|---|---|---|
| 0 · Medir | `0db11c70` | WebP calidad 75: unos 35 KB de media, una séptima parte de lo que llega (tabla en `covers::tests::print_cover_sizes`). libwebp compila en Linux y Android; Windows lo dirá el CI |
| 1 · Reducir | `0db11c70` | 1.000 portadas en 6,7 s en el ordenador, en 10 operaciones |
| 2 · Red local | `7050325e` | Casos hostiles vistos fallar antes del arreglo |
| 3 · Servidor | `ef546646` | 2.108 tests del servidor; en el servidor los casos hostiles se escribieron junto a las reglas, no antes |
| 4 · App con la nube | `f2cf8009` | |
| 5 · Cerrar | este | Limpieza en el dispositivo (contando las portadas de peticiones pendientes), política, guías y arquitectura |

**Para probar en dispositivos:** emparejar por wifi un ordenador y el móvil con portadas en los dos sentidos (el caso de Catán), y dos dispositivos que solo se ven por la nube (el móvil con datos). El servidor necesita el despliegue con la migración nueva antes de la parte de la nube.

## Lo que pasa hoy

- Al guardar una obra, la app baja la portada de su proveedor, la guarda en `covers/<sha256>.<ext>` y la edición apunta a ese nombre (campo `cover`, D27). Hasta 5 MB por imagen; las de BoardGameGeek, por ejemplo, en su tamaño original.
- La sincronización mueve operaciones, nunca ficheros: ni por la red local (`lan/server.rs`: `/v1/sync/pull` y `/v1/sync/push`) ni por la nube (`CloudController`: `pull` y `push`). Quien recibe la edición conoce el nombre de la portada, pero no tiene el fichero ni sabe de dónde salió.
- La copia de seguridad sí lleva las portadas, y al restaurar comprueba el hash (`backup.rs`). Esa comprobación es la que se copia aquí.

## Lo decidido (resumen de D82)

- Viajan los **ficheros**, pedidos por su hash después de las operaciones; nunca se vuelve a buscar ni a bajar de la URL de origen.
- Al guardar: **~800 px por el lado largo, WebP con pérdida (libwebp)**, sin metadatos. Las portadas que ya existen se recomprimen una vez.
- **Red local primero**, en los dos sentidos. **Después la nube**: 200 MB por biblioteca, el servidor no abre las imágenes, y su copia es de trabajo (fuera de la copia diaria del VPS).
- **Tests hostiles antes que cada arreglo.**
- Con datos móviles bajan siempre; «solo con wifi» es una idea del ROADMAP.

## Partes

Cada parte se cierra con sus comprobaciones (`cargo fmt`, Clippy, tests de Rust, `npm run typecheck`, `lint` y `format:check`; en el servidor, `make check`) y con lo que el maintainer tiene que probar. Nada se sube sin su visto bueno.

### Parte 0 · Medir antes de construir

Que WebP compense y que libwebp compile en las cuatro plataformas, antes de apoyar nada en él.

- Dependencias en `app/src-tauri`: `image` (leer JPEG, PNG, WebP y GIF, y reducir) y `webp` (codificar con libwebp, en C). Las dos con licencias que `deny.toml` admite (MIT/Apache, BSD-3-Clause).
- Un test `print_*` (ignorado, a mano) que baja portadas reales de Open Library, Google Books, BoardGameGeek, TMDB y la BNE, y escribe una tabla: tamaño original, JPEG a ~800 px y WebP a ~800 px con varias calidades (70, 75, 80).
- Compilar: escritorio Linux (contenedor), Android (APK de depuración en el contenedor) y Windows (el CI, con *Run workflow* de *Release*, que no publica nada).
- **Sale de aquí:** la calidad elegida y el tamaño medio real. Si libwebp no compila en Android, se para y se decide con el maintainer (por ejemplo, JPEG reducido).
- **El maintainer:** mirar la tabla y, si quiere, comparar a ojo algunas portadas en WebP.

### Parte 1 · Portadas reducidas al guardarlas, y las de antes

Solo la app; la sincronización aún no cambia.

- `Covers::store` normaliza: lee la imagen, la reduce a 800 px por el lado largo (nunca la agranda), la codifica en WebP con la calidad de la parte 0, y el nombre es el hash **del resultado** (`<sha256>.webp`). Volver a codificar quita los metadatos.
- Una portada que ya es WebP, cabe en 800 px y pesa poco se guarda tal cual, para que normalizar dos veces no cambie el hash.
- Límites al leer: tamaño de fichero (los 5 MB de hoy) y **dimensiones máximas antes de descodificar**, para que una imagen trucada no agote la memoria.
- **Las portadas existentes:** una tarea al arrancar la versión nueva recomprime cada fichero de `covers/` al que apunta una edición y registra el cambio de `cover` como operación, por lotes y por biblioteca. Solo donde este dispositivo puede editar; donde solo puede leer, deja la suya (la recibirá de otro). Se marca como hecha en `local_state` para no repetirla.
- Los ficheros viejos se quedan hasta la limpieza (parte 5).
- **Tests:** dimensiones y formato del resultado; sin EXIF (una foto de prueba con GPS); idempotencia; una imagen enorme o rota se rechaza sin descodificarla entera; y en `sync.rs`, con `assert_converged`, dos dispositivos que recomprimen la misma edición a la vez convergen.
- **El maintainer:** actualizar sobre su base de datos de desarrollo (con copia antes) y ver que las portadas siguen bien y ocupan menos.

### Parte 2 · Por la red local, en los dos sentidos

- **Tests hostiles primero**, vistos fallar antes del arreglo (`lan/client.rs`, con el servidor de verdad en local):
  - un dispositivo emparejado pide una portada de una biblioteca que no puede recibir: no la recibe;
  - uno retirado (D71) o con valla (D84, D87): ni bajar ni subir;
  - subir un fichero cuyo contenido no da su hash, o que ninguna edición de las bibliotecas compartidas usa: rechazado;
  - un nombre con rutas (`../`) o con otra forma que `<64 hex>.<ext>`: rechazado antes de tocar el disco.
- **Endpoints nuevos** en el servidor de la red local, con la misma autenticación que `pull` y `push` (`authorized_client`) y las bibliotecas que salen de `shared_libraries`:
  - `POST /v1/covers/inventory`: el cliente manda los nombres que tiene; el servidor contesta cuáles le faltan al cliente y cuáles quiere él. Solo cuenta portadas de ediciones de las bibliotecas compartidas.
  - `GET /v1/covers/{name}` para bajar y `PUT /v1/covers/{name}` para subir. Al subir, el servidor comprueba hash, tamaño y que es una imagen, y la guarda con `Covers::store_exact` (sin recomprimir: el nombre manda).
- **Cuándo:** al acabar cada sincronización (`lan::client::sync`), en segundo plano, cuatro a la vez como mucho. Lo que falle se retoma en la siguiente; no hay estado que guardar.
- **La interfaz:** cuando llega una portada, un evento (`cover-arrived`) hace que el componente `Cover` la vuelva a pedir. Hasta entonces, el hueco de siempre.
- **El maintainer:** el caso de Catán. Emparejar el ordenador y el móvil, y que la portada aparezca en los dos, en ambos sentidos (que la tenga el que muestra el QR y que la tenga el que escanea).

### Parte 3 · El servidor guarda las portadas

- **Tests hostiles primero** (`tests/Controller/`): una cuenta sin acceso a la biblioteca; un dispositivo con valla; hash que no coincide; nombre que no es portada de ninguna edición de esa biblioteca (consulta a `Edition.cover` del catálogo del servidor, D92); tipo que no corresponde a sus bytes; tamaño de más; biblioteca llena (200 MB); nombre con rutas.
- **Rutas**, junto a las de `CloudController` y con `CloudLibraryAccess::require` (nada de fiarse de ids del cliente, D84):
  - `POST /v1/cloud/libraries/{id}/covers/inventory`, como el de la red local;
  - `GET` y `PUT /v1/cloud/libraries/{id}/covers/{name}`.
- **Almacenamiento:** ficheros en `var/covers/<id de la biblioteca en la nube>/<nombre>`, fuera de la carpeta pública, y una tabla con nombre, tamaño y fecha por biblioteca (entidad Doctrine) para el límite y la limpieza. Se escriben a un temporal y se renombran.
- **Sin abrir las imágenes:** tamaño, firma de los bytes (JPEG, PNG, WebP, GIF) y hash.
- **Límites:** 200 MB por biblioteca, un máximo por imagen (512 KB tras la parte 1), y `#[RateLimit]` por cuenta como las demás rutas de la nube (el framework primero).
- **Limpieza:** al borrar una biblioteca de la nube se borran sus portadas; una tarea programada quita las que ya no usa ninguna edición.
- **Copias:** `var/covers` no entra en la copia diaria. Si se pierde, el inventario pide las portadas otra vez a los dispositivos.
- **Documentación:** la guía de autoalojamiento (espacio en disco y que no hace falta copiarlas), `docs/architecture/cloud.md` y `server/AGENTS.md` si cambia alguna regla.

### Parte 4 · La app con la nube

- Los métodos nuevos en el trait `CloudApi` (`covers_inventory`, `download_cover`, `upload_cover`), con `HttpCloud` y el falso de los tests.
- Tras cada sincronización con la nube, el mismo intercambio que en la red local, con el mismo código de transferencia (una sola función que recibe «de dónde» por un trait).
- **Tests** con `FakeCloud`: baja lo que falta, sube lo que el servidor pide, no reintenta lo rechazado en bucle y no manda nada de una biblioteca que no está en la nube.
- **El maintainer:** dos dispositivos que **no** están en la misma wifi (por ejemplo, el móvil con datos), y que la portada llegue por la nube.

### Parte 5 · Cerrar

- **Limpieza en el dispositivo:** las portadas a las que ya no apunta ninguna edición se borran (pendiente desde D27).
- **Textos:** una línea en `whats-new.json`; la política de privacidad (es/en, con fecha): el servidor guarda también las portadas de las bibliotecas que se suben; las guías «Compartir en casa» y «La nube»; `docs/architecture/sync.md` y `data-model.md`.
- La lista de licencias (`npm run licenses`) por las dependencias nuevas.
- El ROADMAP y D82 con lo hecho.

## Después, aparte

- **Fotos propias de portada** (pregunta 2 de D82): con el transporte hecho, activar «Cambiar portada» con la cámara o un fichero; pasan por la misma normalización.
- **Portadas grandes de Google Books** (pregunta 4).
- **«Solo con wifi»** (ROADMAP, ideas para el futuro).

## Riesgos

- **libwebp en Android:** se compila con el NDK. Por eso va la parte 0 antes que nada.
- **La recompresión de la parte 1** registra una operación por cada lote de ediciones: en una biblioteca grande son muchos cambios de una vez. Se mide en la parte 1 con una biblioteca de prueba de 1.000 obras.
- **Cambia `cover` en ediciones que ya están en la nube:** las operaciones suben como cualquier otra; el servidor no necesita nada especial para eso.
