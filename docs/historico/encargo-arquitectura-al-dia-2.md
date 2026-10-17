# Encargo · Más documentos de arquitectura, al día con el código

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Mismo criterio que [encargo-arquitectura-al-dia.md](encargo-arquitectura-al-dia.md):** cambia lo que no coincide con el código y nada más, guarda el porqué, no toques `DECISIONS.md`. **Solo `docs/architecture/cloud.md`, `accounts.md` y `data-model.md`.**

## Qué

Lo que viste al escribir la tercera tanda de «Colaborar»:

1. **`cloud.md`:**
   - el token de avisos dura 10 minutos, no 1 hora;
   - la tabla de rutas: `GET /v1/cloud/libraries` con todo lo que devuelve (`cloudDeviceId`, `cloudPublicKey`, `withdrawn`), y `claim` en la tabla;
   - los límites: `claim` tiene `cloud_link`, y `cloud_events` es de 120 por hora;
   - las invitaciones: la app ya llama a esas rutas;
   - las frases en futuro de lo que ya está hecho.
2. **`accounts.md`:**
   - fuera la cabecera de «hito 7.1, parte 2» y las «partes» futuras que ya existen;
   - `POST /v1/account/code` y el restablecimiento de contraseña, como están hoy;
   - **el segundo factor al entrar con Google:** lo que hace `GoogleController`, es decir, que lo pide. El maintainer decide aparte si se cambia D78.
3. **`data-model.md`:**
   - las columnas que faltan: `members.account_id` y `access_withdrawn`; `devices.trusted_through` y `vouched_key`; `libraries.cloud_*` y `founding_nonce`;
   - `change_requests.status` con `withdrawn`;
   - «Pendiente en hitos posteriores», sin hitos: lo que de verdad falte.

Repasa el resto de los tres documentos con el mismo criterio. **Los identificadores de hallazgos (SEG-…) se pueden quedar** en estos documentos de trabajo: explican el porqué.

## Al terminar

El informe: qué cambiaste en cada documento y lo que no encaje con una decisión.
