# Encargo · Las preguntas frecuentes de la web, iguales que las de la guía

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. **Solo `web/es/preguntas.html` y `web/en/faq.html`.**

## Por qué

Las preguntas frecuentes de la guía ya pasaron por la revisión de jerga y explican más que las de la web:
- `site/src/content/docs/es/using-the-app/faq.md`;
- `site/src/content/docs/en/using-the-app/faq.md`.

La web tiene que decir lo mismo, con las mismas palabras.

## Qué hacer, paso a paso

1. Abre `web/es/preguntas.html` y `site/src/content/docs/es/using-the-app/faq.md`.
2. Para **cada pregunta que está en las dos**, pon en la web el texto de la respuesta de la guía.
   - **Sin los enlaces a otras páginas de la guía** (`../marks/`…): quita el enlace y deja el texto.
   - Los enlaces a la web (`kosmotheka.com/es/…`) sí se quedan.
   - Las negritas de Markdown (`**…**`) pasan a `<strong>…</strong>`.
3. **No cambies** el HTML que rodea las respuestas: ni las etiquetas, ni las clases, ni el orden de las preguntas, ni la cabecera, ni el pie.
4. Las preguntas que solo están en la guía (por ejemplo «¿Cómo borro mi cuenta?»): **añádelas al final**, copiando la estructura HTML de la última pregunta que ya hay.
5. Las preguntas que solo están en la web: **déjalas como están** y apúntalas en el informe.
6. Haz lo mismo con `web/en/faq.html` y `site/src/content/docs/en/using-the-app/faq.md`.

## Al terminar

1. Comprueba que los dos HTML siguen bien formados: `python3 -c "import html.parser,sys; p=html.parser.HTMLParser(); p.feed(open(sys.argv[1]).read())" web/es/preguntas.html` (y lo mismo con el inglés) no da error.
2. **El informe**, corto:
   - cuántas respuestas cambiaste en cada idioma;
   - qué preguntas añadiste;
   - qué preguntas están solo en la web.
