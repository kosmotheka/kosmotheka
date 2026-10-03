# Pruebas manuales de la Fase 7

> Mismo formato que `hito-1.md`: pasos, resultado esperado, `[x]` al pasar. Lo que protege los datos tiene tests automáticos y hostiles (la «Batería de acceso» de `docs/plans/fase-7.md`: Jhonny en PHP, la nube de prueba y los dispositivos simulados en Rust). Aquí va lo que se comprueba con la app delante, contra el servidor de desarrollo.

## Antes de empezar

- **Datos de antes del 27/09:** desde el 03/10/2026 toda operación va firmada (D56) y la nube no acepta historia sin firmar. Si tu instalación es anterior, sus bibliotecas no se podrán subir tal cual: la app ofrecerá «Duplicar como nueva». Para estas pruebas, mejor empezar con carpetas de datos nuevas.
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
