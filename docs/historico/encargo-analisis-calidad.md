# Encargo · Análisis de calidad: lo que el framework ya resuelve

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Solo lee y escribe un informe: no cambia código.** No es una auditoría de seguridad: es un análisis de calidad para el maintainer, que revisará el código con calma. Él quiere el código lo mejor posible, aunque no haya errores, y que sea fácil de mantener.

## El ejemplo que lo ha provocado

En cada acción que lo necesita, el controlador llama a mano a `RequestLimiter` (13 llamadas en 4 controladores). Symfony 8.1 trae `#[RateLimit]` (`vendor/symfony/http-kernel/Attribute/RateLimit.php`, con su `RateLimitAttributeListener`). Su clave puede ser una expresión o un `Closure` evaluado con los argumentos ya resueltos, así que cubre también los límites por cuenta o por la huella del correo del DTO. Solo quedaría a mano el que cuenta únicamente los fallos (`login_failed_account`). Busca más casos así.

**Gasta poco:** busca con `grep` y lee solo lo necesario, sin leer el servidor entero. Antes de proponer un mecanismo, confirma en `vendor/` que existe en la versión instalada: no te fíes de la memoria.

## Parte 1 · Servidor (Symfony 8.1)

Para cada patrón: dónde aparece (`fichero:línea`, todos o los tres primeros más el número total), qué mecanismo del framework o de un bundle ya instalado lo cubriría, y qué se gana.

1. **Límites de peticiones** a mano frente a `#[RateLimit]` (lo de arriba, con el detalle de cada llamada: cuál encaja y cuál no).
2. **Respuestas de error** construidas a mano en los controladores frente a excepciones con su listener o un normalizador (`AccountExceptionListener` ya existe: ¿se usa en todas partes?).
3. **Comprobar quién puede hacer qué** a mano frente a *voters* y `#[IsGranted]`.
4. **Leer el usuario o la petición** a mano frente a `#[CurrentUser]`, `#[MapRequestPayload]`, `#[MapQueryParameter]` y `#[MapUploadedFile]`.
5. **Configuración por entorno** leída a mano frente a `#[Autowire(env: …)]` y `%env()%` con procesadores (`base64:`, `int:`, `bool:`).
6. **Clientes HTTP:** reintentos, tiempos y cabeceras a mano frente a `RetryableHttpClient`, `ScopingHttpClient` y clientes con nombre en `framework.http_client.scoped_clients`.
7. **Caché** a mano (entidades de caché propias) frente a `symfony/cache` con *pools* y etiquetas. Ojo: puede que la caché propia sea a propósito (la huella, sin quién preguntó, la purga): dilo si lo es.
8. **Bloqueos, reloj, UUID, serializar a JSON**: lo hecho a mano que ya tiene componente.
9. **Validación** a mano fuera de `ValidOperation` (que es a mano a propósito, ver `server/AGENTS.md`).
10. **Comandos de consola y tareas programadas**: lo repetido que un atributo o un servicio común resolvería.
11. **Código repetido** entre los cuatro proveedores (TMDB, RAWG, Comic Vine, BGG): ¿hay una base común que lo ahorre sin forzar?

Al final, una sección corta **«No sobreingeniería»**: piezas propias, interfaces con una sola implementación sin necesidad de test, capas que solo pasan llamadas. Solo lo que de verdad sobre.

## Parte 2 · App (Rust y Tauri)

El maintainer no tiene criterio en Rust: aquí el informe tiene que explicar el porqué de cada punto en llano.

1. **Lo mecánico:** `cargo clippy --all-targets -- -W clippy::pedantic` dentro de `library-dev`, sin `-D`. No copies la salida: agrúpala por avisos y cuenta cuántos de cada uno. Di cuáles merecen la pena (legibilidad, errores posibles) y cuáles son ruido en este proyecto.
2. **Patrones a mano** que una crate o la biblioteca estándar ya resuelven: errores (¿`thiserror` en todas partes?), conversiones (`From`/`TryFrom`), `Option` y `Result` con combinadores, SQL repetido que una función común ahorraría, estado de Tauri y comandos repetidos.
3. **Duplicación** entre `crates/domain` y `app/src-tauri`, y entre módulos de proveedores de metadatos.
4. **«No sobreingeniería»**, igual que en el servidor.

## Formato del informe

`docs/research/analisis-calidad-2026-10.md`, en español:

- **Resumen:** cinco líneas con lo más rentable.
- **Por cada parte, una tabla:** `Qué · Dónde · Mecanismo · Ganancia · Riesgo · Esfuerzo (S/M/L)`, ordenada de más a menos rentable. **30 filas como mucho por parte**; lo demás, en una línea al final.
- **Lo que está bien hecho a propósito** y no conviene tocar, en una lista corta, para que el maintainer no lo cambie sin saber por qué.

## Reglas

- No cambies ningún fichero salvo el informe. Nada de `make fix` ni `cargo fix`.
- No lances `make check` ni los tests: no hacen falta.
- Si un punto necesita leer mucho código para decidirse, márcalo «por mirar» y sigue.

Avísame al terminar, con el número de filas de cada parte.
