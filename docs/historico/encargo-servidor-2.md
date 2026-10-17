# Encargo · Montar tu servidor, segunda tanda: usarlo y administrarlo

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Solo `site/src/content/docs/es/run-your-server/`.** La primera tanda explica cómo instalarlo; falta lo que necesita quien ya lo tiene en marcha. Mismas reglas que la documentación anterior:
> - todo comprobado en el código de hoy;
> - sin secretos ni valores de ejemplo reales;
> - nada del servidor oficial (ni su dominio, ni su máquina, ni sus proveedores);
> - sin hitos ni fechas de trabajo.
>
> Lector: alguien que sabe administrar un servidor Linux y no conoce el proyecto. Mira `index.md` y `configuration.md` para el tono.

## Las páginas

1. **`connect-the-app.md` · Conectar la app a tu servidor.**
   - Paso a paso en la app: «Otro servidor», la dirección y qué pasa al crear la cuenta.
   - Qué dice la app si el servidor no responde.
   - Los enlaces de invitación de un servidor propio, que se pegan en la app en lugar de abrirse con un toque (por qué y cómo).
   - Que `http` solo vale en direcciones de la red de casa.

   Nombres de la interfaz tal como salen en `app/src/i18n/locales/es.json`.
2. **`administration.md` · Administrar el servidor.**
   - **Quién puede registrarse:** `REGISTRATION_ALLOWED_EMAILS`, y qué ve quien no está en la lista.
   - **Las órdenes de consola**, una por una (sácalas de `server/src/Command/`), con su uso real, en las dos instalaciones (`php bin/console …` y la de Docker):
     - `app:user:block`;
     - `app:user:two-factor-disable` (cuándo usarla: alguien pierde el móvil y sus códigos de reserva);
     - `app:user:super-admin`;
     - `app:activity:list`;
     - `app:accounts:purge` (lo que borra y cada cuánto lo hace solo el proceso programado);
     - `app:catalog:rebuild` (para qué sirve y cuándo hace falta).
   - **Borrar la cuenta de alguien que lo pide por correo**, si hay forma; si no la hay, dilo y apúntalo en el informe.
3. **`what-the-server-keeps.md` · Qué guarda tu servidor y qué ves tú.**
   - Qué datos guarda:
     - de las cuentas;
     - de los dispositivos;
     - las bibliotecas subidas (sin cifrado de extremo a extremo);
     - el registro de actividad (qué apunta, la IP, un año);
     - la caché de búsquedas (sin quién preguntó ni el texto).
   - **Qué puedes ver tú como administrador**, dicho claro.
   - **Tus obligaciones si alojas a otras personas:** eres tú quien responde de sus datos. Necesitas tu propia política de privacidad; la del proyecto (`web/es/privacidad.html`) sirve de modelo, cambiando el responsable, el alojamiento y el correo. Atiende sus peticiones de acceso y de borrado. Sin asesoramiento legal: dilo así.
   - **Los registros de nginx:** por qué conviene no apuntar las rutas `/v1/metadata/` (llevan lo que se busca) y cómo hacerlo. Si ya está en `with-docker.md` o en `without-docker.md`, enlázalo en lugar de repetirlo.

   Material: `docs/architecture/accounts.md`, `docs/architecture/cloud.md` y la política de privacidad.

Y **`index.md`**: añade las tres páginas en el orden que tenga sentido. Ordena el menú con `sidebar.order`, renumerando lo que haga falta.

## Al terminar

- `cd site && npm run build && npm run check:links`.
- **El informe:**
  - cada página con su número de palabras;
  - lo que no coincidía entre los documentos y el código;
  - y lo que falte en el servidor para administrarlo (por ejemplo, si no hay orden para borrar una cuenta).
