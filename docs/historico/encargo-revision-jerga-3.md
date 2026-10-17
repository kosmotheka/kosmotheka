# Encargo · Revisión de jerga de la guía de usuario (tercera tanda)

> Encargo cerrado, escrito el 17/10/2026. Sin commit, sin push. **Solo lees y escribes un informe; no cambias ningún archivo.**

## Qué leer

Los seis archivos `.md` nuevos de `site/src/content/docs/es/using-the-app/`:

`kinds.md`, `marks.md`, `series.md`, `several-libraries.md`, `data-sources.md`, `faq.md`.

Empieza por `data-sources.md` y `marks.md`.

## Cómo leer

Eres una persona de 60 años que usa WhatsApp y el correo, pero que **no sabe nada de informática**. Lee cada página entera, de arriba abajo.

Apunta **cada palabra o frase que esa persona no entendería** o que le haría dudar. Por ejemplo:
- una palabra técnica sin explicar (formato de archivo, red, servidor, código, clave, proveedor);
- una sigla (ISBN, QR, CSV, wifi está bien);
- una frase larga con dos ideas;
- un paso que da por sabido algo que no se ha dicho (dónde está «el menú», qué es «la ficha»).

No apuntes faltas de estilo ni de gusto: **solo lo que no se entendería.**

## El informe

Escríbelo en `docs/historico/revision-jerga-3.md`, con una tabla por página:

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|

Al final, una línea por página: «bien», «pocos cambios» o «hay que reescribir partes».

No cambies las páginas. No toques nada de `site/`: otra sesión trabaja a la vez en `site/src/content/docs/es/contribute/`. Cuando termines, avisa con un mensaje corto a la sesión `library`.
