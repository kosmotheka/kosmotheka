# Encargo · Avisar cuando la nube no es la oficial

> Encargo cerrado para la sesión de hitos, escrito el 18/10/2026. Sin commit, sin push. Pedido por el maintainer al aceptar la política de marca (D72, TRADEMARKS.md): cualquiera puede montar su servidor y la app oficial se conecta a él, pero **quien usa la app de Google Play tiene que saber cuándo sus datos están en un servidor que no es el oficial** (`https://api.kosmotheka.app`), que lleva otra persona. El caso que más preocupa: **alguien comparte una biblioteca por QR o por invitación, y esa biblioteca vive en la nube de otro servidor.**

## 1. Primero, cuéntame qué hay (sin cambiar nada)

Antes de escribir código, un informe corto:
- **Dónde sabe hoy la app cuál es el servidor oficial** (la dirección por defecto) y dónde enseña el servidor: la pantalla de la cuenta («Otro servidor», «Usar el oficial», «En {{host}}»), la fila de la nube de cada biblioteca («En una nube de otro servidor»)…
- **Qué ve hoy una persona en cada uno de estos casos:**
  1. entra en su cuenta en un servidor que no es el oficial (lo ha escrito a mano);
  2. empareja por QR, en la red de casa, con un dispositivo cuya biblioteca está subida a la nube de otro servidor: ¿qué le llega de ese enlace con la nube, y qué ve?;
  3. acepta una invitación de otro servidor (`invitation_other_server` ya existe: ¿cuándo sale?);
  4. baja de la nube de su cuenta una biblioteca, en una cuenta de otro servidor.
- Para cada caso, **tu propuesta**: dónde y con qué texto avisar, sin jerga. Una línea sencilla, no un muro: por ejemplo «Esta nube no es la oficial de Kosmotheka: la lleva {{host}}».

**Mándame ese informe y para.** El maintainer decide antes de que lo construyas.

## 2. Después (cuando te diga que sigas)

Lo decidido, con:
- una línea en `app/src/whats-new.json` bajo `"0.3.0"` (juntándola con otra si pasas de cinco);
- la guía: `the-cloud.md`, `share-at-home.md` y `run-your-server/connect-the-app.md`, en `es/` y `en/`: qué aviso ve quien usa un servidor que no es el oficial y por qué;
- las comprobaciones de siempre: en `app/`, `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:i18n` y `check:whats-new`; `cargo fmt`, `clippy` y `cargo test` si tocas Rust; en `site/`, `npm run build && npm run check:links`.
