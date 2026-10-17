# Encargo · «Dispositivo», nunca «aparato», y los nombres para quitar un dispositivo

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Va **después** del encargo de la nube por invitación, sin mezclarlos. Solo textos.

## 1. «Dispositivo», siempre

El maintainer: «No uses aparato, usa siempre dispositivo. Aparato es muy raro, no se usa». Hoy «aparato» sale 6 veces en `app/src/i18n/locales/es.json` y unas 99 en `site/src/content/docs/es/`. Cámbialo por «dispositivo», con la concordancia (el/un, -o) y sin dejar frases torpes. Donde se hable de uno en concreto puede seguir diciéndose «móvil» u «ordenador»; nunca «aparato». Mira también `web/` y `docs/publicacion/ficha-google-play.md`.

## 2. Los nombres de las cuatro formas de quitar un dispositivo

Decididos por el maintainer sobre tu propuesta (`docs/research/nombres-quitar-aparato.md`):

| Hoy | Pasa a ser |
|---|---|
| «Olvidar dispositivo» | **«Dejar de emparejar»** |
| «Quitar un dispositivo» (Personas) | **«Retirar el acceso de este dispositivo»** |
| «Quitar» (Mis dispositivos) | **«Sacar de mi cuenta»** |
| «Dejar este dispositivo a otra persona» | **«Dejar este dispositivo como nuevo»** |

- Cambia los botones, títulos y confirmaciones en `es` y en `en`. En inglés, propón tú el equivalente igual de claro: «Unpair», «Remove this device's access», «Remove from my account», «Reset this device»… y dilo en el informe.
- **Las frases de ayuda y confirmación** de tu propuesta, también.
- Cambia los nombres en la Ayuda, en la guía (las dos tandas, `es`, y la primera en `en`) y en la página web de borrar la cuenta, si salen allí.
- Renombra `docs/research/nombres-quitar-aparato.md` a `nombres-quitar-dispositivo.md`, con una línea arriba que diga que se decidió el 17/10/2026 y lo que se eligió.

## 3. Sin préstamos

`people.kick.keeps` dice «se conservan sus préstamos», pero los préstamos aún no existen. Quita esa parte, en `es` y en `en`. Busca si hay más textos de la app o de la guía que den los préstamos por hechos.

## Al terminar

En `app/`: `npm run build`, `typecheck`, `lint`, `format:check` y `knip`. En `site/`: `npm run build`. Un `grep -ric aparato` que dé 0 en `app/src`, `site/src` y `web/`. Una línea en `whats-new.json` bajo `next`, si cabe, para los nombres nuevos.
