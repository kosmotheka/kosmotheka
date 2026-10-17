# Encargo · La primera tanda de la guía de usuario, en inglés

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Solo `site/src/content/docs/en/using-the-app/`.

## Qué

Las siete páginas de la primera tanda, ya revisadas de jerga, en inglés, con el **mismo nombre de fichero** que en español, para que el selector de idioma enlace la misma página:

`what-is-kosmotheka.md`, `first-steps.md`, `add-a-book.md`, `copies-and-places.md`, `share-at-home.md`, `backups.md`, `what-leaves-your-device.md`.

Y el `index.md` de la parte, al día con las páginas.

## Cómo

- **Inglés llano**, para alguien que no programa. Las mismas reglas que en español: frases cortas, de tú, sin jerga.
- **Los nombres de la interfaz, tal como salen en la app en inglés** (`app/src/i18n/locales/en.json`), entre comillas («"Add"», «"Sharing and devices"»). Búscalos en el fichero; no los traduzcas tú.
- Las **mismas palabras fijas** que en español, en su versión inglesa: «library» para lo tuyo, «online services» para lo de fuera, «file», «backup» solo para la copia de seguridad, «copy» para cada ejemplar (como la app), y «pair» definido una vez en «Share at home».
- **Capturas:** las de español sirven si la app sale en español. Si se pueden repetir en inglés con `?mock` cambiando el idioma, mejor; si no, reutiliza las españolas y dilo.
- No es una traducción palabra por palabra: si en inglés se dice mejor de otra forma, dilo así.

## Al terminar

`npm run build` en `site/`, y el informe: las páginas y cualquier texto de la app en inglés que te haya parecido raro o con jerga (apúntalo, no lo cambies).
