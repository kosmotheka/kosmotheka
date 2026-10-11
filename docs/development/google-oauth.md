# «Continuar con Google»: lo que hay que crear en Google Cloud

> Pasos para el maintainer (D77), escritos el 11/10/2026 **antes** de construir el código ([encargo](../historico/encargo-7.1-google.md)). Los nombres de los menús de Google cambian a menudo: si alguno no coincide, busca el concepto (proyecto, pantalla de consentimiento, cliente OAuth). La dirección de vuelta está confirmada con el código (7.1): `DEFAULT_URI` + `/v1/auth/google/callback`.
>
> **Ninguna clave va al repositorio.** El identificador y el secreto del cliente van en el `.env.local` del servidor y en el gestor de contraseñas.

## Qué se necesita y qué no

- **Un proyecto de Google Cloud**, creado con la cuenta del proyecto (el buzón de Gmail del proyecto), no con la personal. El mismo proyecto servirá más adelante para las notificaciones (Firebase, D75).
- **Una pantalla de consentimiento**: lo que ve la persona al elegir su cuenta de Google.
- **Un cliente OAuth de tipo «Aplicación web»**. Aunque Kosmotheka sea una app, quien habla con Google es **el servidor**, así que el cliente es web. No hacen falta clientes de tipo «Android» ni «Escritorio», ni la huella de la firma de la app.
- No hace falta activar ninguna API ni poner datos de pago.

## Pasos

1. **Crear el proyecto.** En <https://console.cloud.google.com/>, con la cuenta del proyecto: selector de proyectos → *Proyecto nuevo* → nombre `Kosmotheka`.
2. **La pantalla de consentimiento** (*Google Auth Platform* → *Branding*, o *APIs y servicios* → *Pantalla de consentimiento de OAuth*):
   - Nombre de la aplicación: `Kosmotheka`.
   - Correo de asistencia: `info@kosmotheka.com`.
   - Tipo de usuarios (*Audience*): **Externos**.
   - Página principal: `https://kosmotheka.com`. Política de privacidad: `https://kosmotheka.com/es/privacidad.html`.
   - Dominios autorizados: `kosmotheka.com` y `kosmotheka.app`.
   - Logotipo: **déjalo vacío por ahora.** Poner logotipo obliga a pasar la verificación de marca de Google antes de que se vea.
3. **Los permisos** (*Data Access* o *Scopes*): solo `openid`, `.../auth/userinfo.email` y `.../auth/userinfo.profile`. Son los básicos: no piden revisión de seguridad.
4. **Estado de publicación: «En pruebas»** (*Testing*) y, en *Usuarios de prueba*, las mismas direcciones que estén en `REGISTRATION_ALLOWED_EMAILS` y sean cuentas de Google. En este estado solo esas personas pueden entrar con Google, que es justo lo que queremos durante la prueba cerrada. Al abrir la nube se pasa a «En producción»; con solo los permisos básicos no hay revisión larga, pero Google puede pedir comprobar que el dominio es nuestro (Search Console).
5. **El cliente** (*Clients* → *Crear cliente*, o *Credenciales* → *Crear credenciales* → *ID de cliente de OAuth*):
   - Tipo: **Aplicación web**. Nombre: `Servidor de Kosmotheka`.
   - *Orígenes de JavaScript autorizados*: vacío.
   - *URI de redireccionamiento autorizados* (**confirmada con el código**: es `DEFAULT_URI` del servidor + `/v1/auth/google/callback`):
     - `https://api.kosmotheka.app/v1/auth/google/callback`
     - para desarrollo en local, otra con la dirección del servidor de desarrollo.
   - Google compara esa dirección letra a letra: `https`, sin barra final, sin parámetros.
6. **Guardar el identificador y el secreto** del cliente en el gestor de contraseñas. El secreto solo se enseña entero al crearlo.
7. **En el servidor**, en `.env.local`:

   ```dotenv
   GOOGLE_CLIENT_ID=<el identificador, acaba en .apps.googleusercontent.com>
   GOOGLE_CLIENT_SECRET=<el secreto>
   ```

   y después `php8.5 bin/console cache:clear`. `GET /v1/server` debe pasar a decir `"sign_in_with_google": true`.

## Cómo comprobarlo

- Con una cuenta de Google que esté en los usuarios de prueba y en `REGISTRATION_ALLOWED_EMAILS`: «Continuar con Google» en la app, elegir la cuenta, y volver a la app ya dentro.
- Con una cuenta de Google que **no** esté en los usuarios de prueba: Google la rechaza antes de llegar al servidor («acceso bloqueado»). Es lo esperado mientras esté «En pruebas».
- `redirect_uri_mismatch` en el navegador: la dirección de vuelta del paso 5 no coincide con la que manda el servidor. El mensaje de Google enseña la que recibió: se copia tal cual al cliente.

## Lo que Google nos da y lo que guardamos

De Google solo se recibe el identificador de la cuenta, el correo (y si está verificado) y el nombre. No se guarda ningún token de Google ni se pide acceso a nada más. Va en la política de privacidad.

## Para quien monte su propio servidor

Todo esto es opcional: sin `GOOGLE_CLIENT_ID`, el servidor no ofrece Google y la app no enseña el botón.
