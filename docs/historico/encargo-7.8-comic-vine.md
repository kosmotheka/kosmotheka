# Encargo 7.8 (quinta parte) · Cómics con Comic Vine

> Encargo cerrado para la sesión de hitos, 12/10/2026 ([plan](fase-7.md), «7.8 Proveedores con clave»). Sin commit, sin push, sin cambiar de rama. Toca `server/`, `app/`, `docs/` y **una línea** de `crates/domain` (la lista de sitios de portadas). Nada en `spec/`.

## Para qué

Los cómics solo se buscan hoy en Wikidata. Comic Vine tiene series y números. Es el último proveedor con clave previsto mientras BoardGameGeek no conteste, y debe salir **completo de una vez**: por el servidor para quien tiene cuenta, y con clave propia para quien no (D62). Sigue lo hecho con RAWG ([tercera](../historico/encargo-7.8-rawg.md) y [cuarta parte](../historico/encargo-7.8-claves-propias-y-todas.md)): mismas piezas, mismos cuidados.

## Condiciones de Comic Vine (releídas el 12/10/2026 en `comicvine.gamespot.com/api/`)

- **Estrictamente no comercial**; si no, revocan la clave.
- **200 peticiones por recurso y hora**, con detección de ráfagas. Recomiendan guardar las respuestas.
- **Enlace de vuelta en cada página donde se usen sus datos.**
- Prohíben productos que compitan con ellos y redistribuir sus datos alterados.

**Lee su documentación** (`comicvine.gamespot.com/api/documentation`) antes de escribir nada: recursos, campos, cómo va la clave, si exigen un `User-Agent` propio. Si no puedes leerla, **dímelo en el informe** y marca qué has supuesto. Lo que yo recuerdo, sin garantía: la clave va como parámetro `api_key` de la dirección, hay que pedir `format=json`, la búsqueda es el recurso `search` con `resources=volume,issue`, y rechazan los agentes de usuario genéricos.

## Qué se busca

Por título, para el tipo `Comic`:

- **Series** (su recurso `volume`): nombre, año de inicio, editorial, portada.
- **Números** (`issue`): nombre de la serie, número, título del número si lo tiene, año de la fecha de portada, portada. El candidato lleva `number` y el formato de número suelto que ya usa Wikidata para cómics.
- Autores: del detalle, **solo para los tres primeros resultados**. Mira qué papeles existen en `ContributorRole` (guion, dibujo…) y usa los que encajen; lo que no encaje, se omite. Ningún papel nuevo.

Hasta 8 candidatos entre series y números; decide el reparto y el orden y dímelo.

## Servidor

1. `POST /v1/metadata/comic-vine/search` con sesión; cuerpo `{query}` (2 a 100 caracteres tras limpiar). Respuesta propia con la forma fija.
2. Clave en `COMIC_VINE_API_KEY`, vacía por defecto. Si solo se puede mandar en la dirección, **todo lo de RAWG**: cliente HTTP aparte sin registrador, excepciones sin `previous`, descartar textos que contengan la clave, y el test de la clave centinela sobre respuestas, filas y log.
3. Tiempo máximo, tope de lectura, sin redirecciones, reintento ante `5xx`, freno ante `429`/`401`/`403`, por `ProviderGate`.
4. **Límites, aquí por hora y por recurso:** 150 llamadas reales por hora a la búsqueda, 150 al detalle de series y 150 al de números (por debajo de sus 200). El de 600 por hora por cuenta, compartido con los demás.
5. Caché en la misma tabla: búsqueda 7 días bajo huella, detalle 30 días por su número. Sin quién preguntó, sin el texto en claro.
6. `/v1/server` añade `"comic_vine"` a `metadata`. No se registra qué busca cada cuenta.

## App

1. `Source::ComicVine`, en `Lookup::search` para `Comic`, junto a Wikidata y primero.
2. **Clave propia desde el principio:** en `KEYED_PROVIDERS`, con su campo, «Probar» y «Cómo conseguir una clave» en Proveedores. Orden: clave propia (directo, nada hacia el servidor) → servidor → nadie. La respuesta directa pasa por el mismo lector que la del servidor.
3. Portadas: su sitio de imágenes, que confirmarás en la documentación (añádelo a `COVER_HOSTS`; es lo único de `crates/domain`). Solo `https`, solo ese sitio, solo extensiones de imagen que la app sabe guardar.
4. Proveedores: aparece en «Todas» y en «Cómics», con las mismas reglas que RAWG.
5. **Atribución:** en «Créditos», Comic Vine con enlace a `https://comicvine.gamespot.com`; en la pantalla de resultados, cuando haya alguno suyo, una línea «Datos de Comic Vine» con el enlace. Si ya hay una de RAWG, que la pieza sirva para las dos sin duplicar código. Nada en las fichas.
6. Cadenas en `en` y `es`. Tests sin red. No reintroduzcas `min-h-0` en el diálogo.

## Casos hostiles (test antes de la defensa, visto fallar; enséñame el rojo del servidor y el de la app)

Los trece del encargo de RAWG, adaptados, más los cinco de la clave propia de la cuarta parte, y:

19. Un número de serie o de número que no sea un entero positivo razonable nunca llega a una dirección.
20. Un resultado de un recurso que no se pidió (personaje, equipo, editorial…) se descarta.
21. Agotado el tope de un recurso (por ejemplo, el detalle de números), los demás siguen funcionando.

## Reglas

- Doctrine primero; consultas con parámetros. Sin dependencias nuevas.
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.
- Tu apunte en `docs/plans/fase-7.md`, con una inserción puntual, sin reescribir el fichero.

## Al terminar

Servidor: `make check` **después del último cambio** y `make migration`. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. La salida real de cada uno.

Documentación: `docs/architecture/cloud.md`, `docs/research/proveedores.md`, `docs/publicacion/servidor-produccion.md`, `docs/publicacion/politica-privacidad-con-servidor.md`, `docs/plans/fase-7.md` y `docs/ROADMAP.md`.

Informe: qué has hecho, qué decidiste tú y por qué, qué leíste de su documentación y qué supusiste, qué tests viste fallar antes, qué no has podido comprobar y cualquier cosa rara.
