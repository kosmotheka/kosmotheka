# Pruebas manuales de la Fase 7

> Mismo formato que `hito-1.md`: pasos, resultado esperado, `[x]` al pasar. Lo que protege los datos tiene tests automáticos y hostiles (la «Batería de acceso» de `docs/plans/fase-7.md`: Jhonny en PHP, la nube de prueba y los dispositivos simulados en Rust). Aquí va lo que se comprueba con la app delante, contra el servidor de desarrollo.

## Antes de empezar

- **Datos de antes del 04/10/2026:** desde el 03/10 toda operación va firmada (D56) y desde el 04/10 cada una encadena la anterior de su autor (D87). La historia anterior no viaja: la app ofrecerá «Duplicar como nueva». Para estas pruebas, empieza con carpetas de datos nuevas (o con `make app-fixtures`).
- **Servidor:** `cd server && make up && make migrate`. La API queda en `http://localhost:8095` y el correo en Mailpit (`docs/development/server.md`).
- **App:** `cd app && npm run tauri dev`. Cada instancia extra se abre con su propia carpeta: `KOSMOTHEKA_DATA_DIR=/tmp/kosmotheka-b target/debug/kosmotheka` (ver `fase-2.md`).
- **Iniciar sesión contra el servidor local:** en la cuenta, «Iniciar sesión» → «Otro servidor» → `http://localhost:8095`.
- **Con los personajes ya hechos:** `cd server && make app-fixtures` carga las fixtures del servidor y prepara las carpetas de los dispositivos de Ana, Hugo, Kenji y Jhonny, ya con la sesión iniciada. Qué abrir y qué esperar en cada una: `docs/development/fixtures.md`, «Abrir los dispositivos en la app».

## 7.2 · Sincronizar con la nube

### Subir

- [ ] **Sin sesión no se sube.** Sin iniciar sesión, en una biblioteca → Ajustes de la biblioteca. *Esperado:* la fila de la nube dice que hace falta iniciar sesión; nada más cambia.
- [ ] **Elegir qué subir.** Inicia sesión (A, «Portátil de Ana») → cuenta → «Bibliotecas en la nube». *Esperado:* cada biblioteca con su estado; las que puedes compartir, marcadas; ninguna sale sin marcarla.
- [ ] **Subir.** Sube «Casa». *Esperado:* la fila pasa a «Al día»; en `make console` o en tu cliente de base de datos, una fila nueva en `cloud_library` con el `local_library_id` de «Casa» y sus operaciones en `cloud_operation`.
- [ ] **Subir dos veces no hace nada.** Vuelve al selector. *Esperado:* «Casa» aparece como ya en la nube, sin casilla.
- [ ] **Un lector no sube.** Con roles activados, en el dispositivo de un lector. *Esperado:* «Eres lector: la sube quien administra»; no hay botón.
- [ ] **Historia sin firmar.** En una instalación anterior al 27/09 (o la biblioteca «Recetas de la abuela» del modo `?mock&account&cloud=on`). *Esperado:* la explicación y «Duplicar como nueva»; la copia queda marcada y se sube; la original sigue igual en este dispositivo.

### Otro dispositivo de la misma cuenta

- [ ] **Bajar sola.** Abre B con una carpeta nueva e inicia sesión con la misma cuenta. *Esperado:* tras sincronizar, «Casa» aparece con sus libros, sin emparejar nada.
- [ ] **Ida y vuelta.** Añade un libro en B y sincroniza; luego sincroniza A. *Esperado:* el libro está en A.
- [ ] **Sin conexión no bloquea.** Para el servidor (`make down`) y añade libros en A. *Esperado:* se añaden como siempre; el indicador dice «Sin conexión con la nube». Arranca el servidor y «Sincronizar ahora». *Esperado:* llegan a B.

### La nube y la red local a la vez (D39)

- [ ] **Por los dos caminos.** Empareja A y B por la red local compartiendo «Casa» además de la nube. Añade un libro en B, sincroniza B con la nube y después A con B por la red local. *Esperado:* el libro está una sola vez en A.
- [ ] **El móvil que la tenía antes de subirla.** C recibe «Casa» por la red local; después A la sube; C, con la misma cuenta y **sin** red local, sincroniza con la nube. *Esperado:* C queda enlazada y al día, sin aviso de conflicto.

### Desconocidos (D84)

- [ ] **El móvil nuevo de la misma cuenta (D85).** Con la cuenta de Kenji, A sube su primera biblioteca. Abre una carpeta nueva (su primera biblioteca tiene otro id, aleatorio), inicia sesión con Kenji y sincroniza. *Esperado:* baja la biblioteca de Kenji con sus libros, sin aviso de conflicto, y la suya vacía sigue ahí.
- [ ] **Un id repetido, sin mezclar.** Desde la app ya no se puede provocar (cada biblioteca nace con id aleatorio, D85): lo cubren los tests (`a_library_with_the_same_local_id_that_was_never_linked_is_not_merged`) y las fixtures del servidor, que usan a propósito un id que dos desconocidos comparten.
- [ ] **Lo que no se ve en pantalla** (Jhonny con los ids de otros, firmas falsas, un id de dispositivo ajeno): lo prueban los tests hostiles y se ve en la base tras `make fixtures` (`docs/development/fixtures.md`).

### Avisos

- [ ] **Biblioteca que ya no está en la nube.** Borra en la base la fila de `cloud_library` de una biblioteca subida y sincroniza. *Esperado:* el aviso de que ya no está en la nube; la biblioteca sigue en el dispositivo.

## 7.3 · Avisos al momento

Hace falta el servidor con el hub: `cd server && make up` (el contenedor de Mercure es `mercure_kosmotheka`). Dos dispositivos con la **misma cuenta** (A y B, cada uno con su carpeta de datos) y «Casa» ya subida y bajada en los dos.

### Entre dos dispositivos de la cuenta

- [ ] **Un cambio llega solo.** Con A y B abiertos, añade un libro en A y no toques nada en B. *Esperado:* en B aparece el libro en unos segundos, sin sincronizar a mano.
- [ ] **El otro sentido.** Edita ese libro en B. *Esperado:* el cambio aparece en A en unos segundos.
- [ ] **Una ráfaga.** En A añade cinco libros seguidos. *Esperado:* llegan todos a B; no se nota que sincronice cinco veces.
- [ ] **La campanita.** Con roles en «Casa» y B administrando, haz en A (como lector) una petición. *Esperado:* el contador de la campanita de B sube sin tocar nada.
- [ ] **El indicador.** Con sesión iniciada y el hub en marcha. *Esperado:* junto al estado de sincronización, una marca «En directo» (al pasar el cursor: «Los cambios de la nube llegan al momento»). Sin sesión, no hay marca.

### Sin hub

- [ ] **Parar el hub.** `docker stop mercure_kosmotheka`. *Esperado:* en unos segundos la marca pasa a «Sin conexión en directo». La app sigue funcionando.
- [ ] **Sin hub, se sincroniza igual.** Con el hub parado, añade un libro en A. *Esperado:* a los pocos segundos sale de A hacia la nube (el cambio propio sincroniza a los 2 s); en B aparece al abrir la app o al volverla a primer plano, no antes.
- [ ] **Volver a arrancarlo.** `docker start mercure_kosmotheka`. *Esperado:* en menos de un minuto la marca vuelve a «En directo» y B recibe lo que se hizo mientras tanto, sin tocar nada.
- [ ] **Servidor sin hub.** Con un servidor que diga `events: false` en `GET /v1/server`. *Esperado:* no hay marca y la app se comporta como antes de este hito.

### En el móvil

- [ ] **Modo avión.** Con el móvil como B: activa el modo avión, haz un cambio en A, desactívalo. *Esperado:* al recuperar la red, la marca vuelve a «En directo» y el cambio llega solo.
- [ ] **Volver a primer plano.** Deja la app en segundo plano unos minutos y vuelve a ella. *Esperado:* la marca está en «En directo» (o lo está en pocos segundos) y se pone al día.

### Red local

- [ ] **De servidor a cliente.** Empareja A y B por la red local (A sirve, B se conecta). Añade un libro en A. *Esperado:* con la app de B en primer plano y sin tocarla, el libro llega en 5 minutos como mucho; antes si B se abre, vuelve a primer plano o hace un cambio propio. No hay aviso de servidor a cliente (D88).
- [ ] **De cliente a servidor.** Añade un libro en B, el cliente. *Esperado:* a los 2 segundos B lo envía y en A aparece sin tocar nada (la pantalla de A se recarga sola).
- [ ] **Escanear una tanda en el móvil.** Sin sesión en la nube, el portátil muestra el QR y el móvil lo escanea (el portátil sirve). Con la app abierta en los dos, escanea cinco libros seguidos en el móvil sin cerrarla. *Esperado:* van apareciendo en el portátil mientras escaneas, sin tocarlo y sin cerrar ni volver a abrir la app del móvil. Si a la vez editas un libro en el portátil, el móvil lo recibe en su siguiente escaneo.
- [ ] **Un móvil de puente.** El portátil sirve a tu móvil (con sesión en la nube) y tu móvil sirve al de tu hijo (sin cuenta). Añade un libro en el móvil de tu hijo. *Esperado:* llega a tu móvil a los 2 segundos y de ahí, sin tocar nada, al portátil y a la nube (y a otro dispositivo de tu cuenta).
- [ ] **En segundo plano no pregunta.** Deja la app del cliente en segundo plano más de 5 minutos y añade un libro en el servidor. *Esperado:* no llega hasta que el cliente vuelve a primer plano (y entonces llega al momento).

### Sesión

- [ ] **Cerrar sesión.** Cierra sesión en A. *Esperado:* la marca desaparece y no queda ninguna conexión abierta (`docker logs mercure_kosmotheka` no muestra suscriptores de A).
- [ ] **Quitar un dispositivo.** Desde B, quita A de la cuenta. *Esperado:* A se reinicia como una instalación nueva y deja de escuchar.

## 7.8 · Proveedores con clave (12/10/2026)

Dos tandas, en este orden: la primera no necesita desplegar nada; la segunda, sí.

### Tanda 1 · En local, con la app de desarrollo (`npm run tauri dev`)

**La pantalla de Proveedores** (Ajustes › Proveedores de datos):

- [ ] Al abrir, **«Todas»** está seleccionada y a la derecha sale cada fuente **una sola vez**, con los tipos que cubre debajo.
- [ ] Al pulsar un tipo (Libros, Películas, Videojuegos, Cómics), la lista se filtra; al volver a «Todas», salen todas.
- [ ] Apaga una fuente en «Todas» y mira que sale apagada también dentro de su tipo. Vuelve a encenderla.
- [ ] Al final, tras hacer scroll, está **«Créditos»**: logo y aviso de TMDB, y RAWG y Comic Vine con su enlace. **No se monta encima de la lista** en ningún momento del scroll.
- [ ] Los enlaces de «Créditos» y los de «Cómo conseguir una clave» abren el navegador.
- [ ] Estrecha la ventana hasta el ancho de un móvil: primero la lista de tipos con «Todas» arriba; al entrar en uno, sus fuentes; el campo de clave cabe.
- [ ] Modo claro (Ajustes › Apariencia): el logo de TMDB y los textos se leen.

**Clave propia** (cierra sesión antes, para que nada pase por el servidor):

- [ ] **TMDB:** pega el «API Read Access Token» (el largo), «Probar». *Esperado:* dice que funciona. Busca una película y una serie: salen resultados con la etiqueta TMDB, director o creadores, duración o temporadas y cartel.
- [ ] **TMDB con una clave mala:** cambia un carácter y «Probar». *Esperado:* aviso de clave incorrecta, no un error genérico.
- [ ] **Una película grande** (una de Marvel): sale el director.
- [ ] **RAWG:** pega la clave, «Probar», busca un videojuego. *Esperado:* resultados con la etiqueta RAWG; los tres primeros con estudio y editor; plataforma rellena solo si el juego tiene una; portada; y debajo de los resultados la línea «Datos de RAWG» con su enlace. **Es la primera vez que se prueba contra RAWG de verdad:** si no sale nada o salen campos vacíos, apunta qué buscaste.
- [ ] **Comic Vine** (solo con clave propia; el servidor oficial no la pone, D62): pega la clave, «Probar», busca una serie («batman») y un número («watchmen 1»). *Esperado:* series y números, con editorial y año; **portadas** (si no sale ninguna, el sitio de sus imágenes no es el que supusimos); autores en los números. Línea «Datos de Comic Vine».
- [ ] Borra las claves y, sin sesión, busca una película. *Esperado:* solo Wikidata, y Proveedores explica que TMDB pide clave o cuenta.

### Tanda 2 · Por el servidor (tras desplegar RAWG; Comic Vine no se despliega con clave)

- [ ] `curl -s https://api.kosmotheka.app/v1/server` lista en `metadata` lo desplegado (`google_books`, `tmdb`, `rawg`; `comic_vine` no debe salir).
- [ ] Con sesión y **sin** claves propias: busca un videojuego. *Esperado:* resultados de RAWG.
- [ ] En el VPS, `sudo -u postgres psql <la base de datos> -c "SELECT provider, left(code,12), found FROM metadata_cache ORDER BY fetched_at DESC LIMIT 10;"` enseña filas `rawg_search` y `rawg_game`, sin el título buscado en claro.
- [ ] Repite la misma búsqueda. *Esperado:* responde igual y no hay filas nuevas (salió de la caché).
- [ ] Con sesión **y** clave propia de TMDB puesta: busca una película y comprueba que no aparece ninguna fila nueva `tmdb_search` (la app fue directa a TMDB).
- [ ] En el VPS, `grep -c "metadata" <el registro de nginx>` da `0`.

## Library of Congress (13/10/2026)

En local, con la app de desarrollo. No hay nada que desplegar.

- [ ] En Ajustes › Proveedores de datos, filtro «Libros»: aparece «Library of Congress», encendida, sin pedir clave ni cuenta.
- [ ] Añadir por ISBN un libro en inglés (`9780756404734`): se rellenan autoría, editorial y año, y entre las fuentes sale la Library of Congress.
- [ ] Buscar por título «the name of the wind» en Libros: salen resultados de la Library of Congress, todos libros (ningún disco ni fotografía).
- [ ] Apagarla en Proveedores y repetir: ya no sale.
- [ ] Un ISBN español (`9788401339639`) sigue saliendo con los datos de la BNE.
- [ ] En el móvil, lo mismo con un ISBN en inglés.
