# Encargo · Guía de usuario, tercera tanda: el catálogo por dentro

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Mismas reglas de escritura y mismas palabras decididas** que en la primera y la segunda tanda: léelas antes en [encargo-guia-usuario-1.md](encargo-guia-usuario-1.md), [encargo-guia-usuario-1-jerga.md](encargo-guia-usuario-1-jerga.md) y [encargo-guia-usuario-2.md](encargo-guia-usuario-2.md). Mira también una página ya revisada, como `site/src/content/docs/es/using-the-app/add-a-book.md`, para el tono. Solo `site/src/content/docs/es/using-the-app/` y sus imágenes. Nada de cambiar la app.

## Palabras fijas (recordatorio)

- «dispositivo», **nunca «aparato»**;
- «archivo», no «fichero»;
- «biblioteca» para lo tuyo, «ejemplar» para cada cosa de la estantería, «servicios de internet» o «catálogos públicos» para lo de fuera;
- «copia de seguridad» solo para eso;
- «emparejar», ya definido en «Compartir en casa»: aquí se enlaza, no se define otra vez;
- los nombres de los botones, **tal como salen en la app** (`app/src/i18n/locales/es.json`), entre comillas angulares.

## Las páginas

Cada una responde a una pregunta que alguien se hace con la app en la mano. Nombres de archivo en inglés, que se repetirán igual en `en/`.

1. **`kinds.md` · Libros, cómics, juegos, películas, música: qué cambia en cada tipo.**
   - Qué tipos hay.
   - Qué campos propios tiene cada uno: jugadores y duración de un juego de mesa, temporadas de una serie, plataforma de un videojuego…
   - Los formatos, físicos y digitales («lo digital no tiene estantería»).
   - Los géneros de cada tipo.
   - Material: `crates/domain` y la ficha en la app; `docs/testing/fase-6.md`.
2. **`marks.md` · Lo de cada uno: por leer, leyendo, leído, estrellas, favoritos y «lo deseo».**
   - Que cada persona apunta lo suyo sobre la misma obra.
   - Los nombres de los estados por tipo («jugado», «visto»…).
   - «Lo deseo» y «me falta»: qué es cada uno, dónde se ven y en qué se diferencian. Míralo en el código, no en los planes.
   - Filtrar por eso.
3. **`series.md` · Series y huecos.** Agrupar obras en una serie, su número y los huecos de lo que falta.
4. **`several-libraries.md` · Varias bibliotecas.**
   - Para qué sirven: la de casa, la del club, la tuya.
   - Crear una, cambiar de una a otra, «en todas», mover una obra y borrar una (con su copia antes).
   - Que cada una se comparte por separado.
5. **`data-sources.md` · De dónde salen los datos: catálogos públicos, claves propias y cómo apagarlos.**
   - La pantalla «Proveedores de datos»: cuáles hay por tipo, cuáles necesitan clave, poner una clave propia y apagar uno.
   - Qué se envía a cada uno: lo que buscas, nada más.
   - Los que van a través del servidor si tienes cuenta.
   - Que coincida con la política (`web/es/privacidad.html`) y con «Qué sale de tu dispositivo».
6. **`faq.md` · Preguntas frecuentes.**
   - Las de `web/es/preguntas.html`, ya al día, pasadas a la guía.
   - Cada respuesta corta, con enlace a su página de la guía cuando la haya.
   - Que digan lo mismo que la web. Si ves que una de las dos está mal, apúntalo y no la cambies.

Y **`index.md`**: hoy solo lista la primera tanda y dice que «todavía faltan» la nube y las personas, que ya existen. Ponlo al día con todas las páginas, en un orden que tenga sentido para quien empieza. El menú lateral se genera solo: ordénalo con `sidebar.order` en cada página, sin tocar `astro.config.mjs`.

## Lo que pesa en esta tanda

- **Todo sale de la app tal como es hoy**, comprobado en el código y en `?mock`, no de los planes ni del diseño. Si el código y el texto de la interfaz no coinciden, o algo te parece un fallo, **apúntamelo en el informe, no lo arregles**.
- **Nada de lo que no existe:**
  - ni préstamos ni reservas de regalos;
  - ni foto de estantería ni IA;
  - ni avisos con la app cerrada.

  Si una página necesita decir que algo no está, una línea basta, sin prometer fechas.
- Capturas solo donde ayuden, de `?mock`, con nombres inventados («Ana», «Laura», «Hugo»).

## Al terminar

- `npm run build` en `site/`.
- **El informe:**
  - para cada página, su título, cuántas palabras tiene y la jerga encontrada y cambiada;
  - la lista de diferencias o posibles fallos entre lo que la app dice y lo que hace;
  - y lo que no coincida entre `faq.md` y la web.
