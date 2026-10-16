# Encargo · Aplicar la revisión de jerga a la guía (primera tanda)

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push. Sigue a [revision-jerga-1.md](revision-jerga-1.md), que hizo `library-haiku` leyendo como alguien que no sabe de informática. Toca `site/src/content/docs/es/using-the-app/` y seis textos de la app.

## Lo que decide la sesión coordinadora sobre las palabras

Son las dudas del final del informe:

1. **Tus cosas:** «tu biblioteca» o «tu catálogo», como dice la app. **Lo de fuera:** «servicios de internet» que buscan los datos (de libros, películas…), y, cuando haya que ir a la pantalla, su nombre tal cual, «Proveedores». Nunca «catálogo» para lo de fuera.
2. **Un solo nombre para lo de fuera** en las siete páginas: «servicios de internet». La primera vez se añade un ejemplo: «como el catálogo de la Biblioteca Nacional».
3. **«Archivo», no «fichero».** Es lo que dice casi todo el mundo. **Se cambia también en la app**: las 6 apariciones de «fichero» en `app/src/i18n/locales/es.json` pasan a «archivo», con la concordancia que haga falta. El inglés no cambia.
4. **«Copia»** solo para la copia de seguridad. Cada libro que tienes es un «ejemplar», como en la app.
5. **«Emparejar»** se define una vez, al principio de «Compartir en casa»: «emparejar es conectar dos aparatos para que se pasen los cambios». Las demás páginas enlazan ahí.
6. **Unidades** (`copies-and-places.md`, línea 26 del informe) y **borrar una biblioteca** (`what-leaves-your-device.md`, línea 66): **compruébalo en el código** y escribe lo que hace la app de verdad. Dime qué encontraste.

## El resto del informe

Aplica las filas de las tablas con criterio. Si una propuesta del informe alarga mucho una frase o se equivoca sobre la app, déjala y dime por qué en una línea. Las dos páginas para reescribir por partes son «Compartir en casa» y «Copias de seguridad»: ahí el objetivo es que cada paso se entienda sin saber nada de antes.

## Al terminar

- `npm run build` en `site/`; en `app/`, `npm run typecheck`, `lint` y `format:check`.
- Un informe corto: cuántas filas aplicaste y cuántas no (con el porqué), lo que encontraste de las unidades y del borrado, y los seis textos de la app que cambiaron.
