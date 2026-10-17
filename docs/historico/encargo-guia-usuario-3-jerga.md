# Encargo · Aplicar la revisión de jerga a la tercera tanda

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Sigue a [revision-jerga-3.md](revision-jerga-3.md), que hizo `library-haiku` leyendo como alguien que no sabe de informática. Va **después** de [encargo-colaborar-1.md](encargo-colaborar-1.md), sin mezclarlos. Solo `site/src/content/docs/es/using-the-app/`: las seis páginas de la tercera tanda y, si hace falta, el glosario de la parte 3.

## Lo que decide la sesión coordinadora

1. **Los nombres de la interfaz no se cambian** aunque el informe lo proponga. «Dirección», «Diseño», «Sello», «Referencia», «Grapa», «Eliminar serie», «Guardar en Me falta» y los demás: si así sale en la app, así va, entre comillas. Se puede añadir una aclaración corta detrás («Dirección», quien dirige la película), sin cambiar el nombre.
2. **Las palabras ya decididas no se cambian:**
   - «dispositivo», nunca «aparato» (el informe lo usa en una propuesta: no la copies);
   - «servicios de internet» para lo de fuera;
   - «archivo»;
   - «emparejar», enlazado a «Compartir en casa».
   - «Servidor» no pasa a ser «el ordenador de Kosmotheka». Donde haga falta, la primera vez en la página: «el servidor (el ordenador en internet que guarda las bibliotecas de la nube)».
3. **«Obra», «ficha» y «ejemplar»** salen en casi todas las páginas. **Un apartado corto «Tres palabras de la app»** en `kinds.md`, que es donde se empieza el catálogo. En las demás páginas, la primera vez, un enlace a ese apartado. No se explican de nuevo en cada página.
4. **Las listas de géneros** (`kinds.md`) se quedan como salen en la app. Encima, una línea: son los de la app, y se puede elegir cualquiera.
5. **`faq.md`, la frase de la AGPL:** el repositorio aún no es público, así que hoy no se puede leer el código. Escríbela para hoy: «Será software libre (licencia AGPL): cualquiera podrá leer el código, cambiarlo y compartirlo». La misma idea, en el mismo tiempo verbal, que en `what-is-kosmotheka.md`; mira cómo lo dice allí.
6. **«Catalogar»** pasa a «apuntar» en `faq.md`, como en el resto de la guía.
7. **`marks.md`, «En curso» frente a «Empezado»:** mira en `es.json` cuál sale y cuándo, y escribe lo que hace la app.

## El resto del informe

Aplica las filas con criterio. Si una propuesta alarga mucho la frase o se equivoca sobre la app, déjala y di por qué en una línea. Las dos páginas para reescribir por partes son `data-sources.md` y `several-libraries.md`: que cada paso se entienda sin saber nada de antes, y que en cada frase haya una sola idea.

**La captura `data-sources-dialog.webp`** enseña el texto viejo («con la nube, todas funcionan sin claves»). Ese texto ya se corrigió en la app (commit `9ba9f49`): vuelve a hacerla desde `?mock`.

## Al terminar

- `cd site && npm run build && npm run check:links`.
- **El informe**, corto:
  - cuántas filas aplicaste y cuántas no, con el porqué de las que no;
  - qué encontraste de «En curso».
