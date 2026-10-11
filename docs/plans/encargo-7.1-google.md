# Encargo · «Continuar con Google» (D77)

> Encargo cerrado para la sesión de hitos, 11/10/2026. **Va después de 7.6.4** (necesita que un enlace abra la app). Sin commit, sin push, sin cambiar de rama. Es la pieza de 7.1 que quedó sin hacer. Toca la puerta de entrada de todas las cuentas: léelo entero antes de empezar y, si algo no encaja con el código, **para y dímelo**.

## Qué leer

D77, D49, D50, D78 y D66 en `docs/DECISIONS.md`; `docs/architecture/accounts.md` entero (el reto de inicio de sesión, el token del dispositivo, `no_password`, el segundo factor, «Qué ofrece el servidor»); `docs/plans/fase-7.md`, 7.1; y cómo quedó el punto de entrada de enlaces de 7.6.4. Mira la documentación actual de `knpuniversity/oauth2-client-bundle` y de `league/oauth2-google` antes de escribir.

## El camino (propuesta del coordinador sobre D77; lo que elijas distinto, dímelo)

D77 fija: por el navegador del sistema, el servidor hace el OAuth con PKCE y devuelve a la app **un código de un solo uso** que la app cambia por el token de su dispositivo. Concretado:

1. **La app empieza.** Genera un secreto aleatorio (el «verificador») que no sale de la app, y llama a `POST /v1/auth/google/start` con su huella (SHA-256). El servidor guarda un intento (id aleatorio, la huella, caducidad de 10 minutos, un solo uso) y devuelve la dirección que hay que abrir en el navegador.
2. **El navegador** va a esa dirección del servidor, que redirige a Google (con su propio `state` y PKCE entre servidor y Google). Google devuelve al servidor. El servidor valida la respuesta y el `id_token`, y **solo sigue si Google dice `email_verified`**.
3. **El servidor decide la cuenta** (reglas abajo), crea un código de un solo uso atado a ese intento (dos minutos de vida, solo se guarda su huella) y manda el navegador de vuelta a la app con el esquema propio: `kosmotheka://auth/google?attempt=…&code=…`. En el navegador queda una página que dice «Ya puedes volver a Kosmotheka».
4. **La app termina.** Solo acepta una vuelta cuyo intento sea **el que ella misma empezó** y tiene pendiente. Llama a `POST /v1/auth/google/finish` con el código, el verificador y **la misma prueba de dispositivo que un inicio de sesión normal** (reto firmado con su clave). El servidor comprueba código, verificador y prueba, y da el token del dispositivo como en `/login`.

Por qué así: el código que viaja por el navegador y por el enlace **no sirve sin el verificador**, que nunca salió de la app. Otra app que registre el mismo esquema y robe la vuelta no consigue nada, y por eso vale el esquema propio en todas las plataformas (también en servidores propios), sin depender de enlaces verificados.

## Qué cuenta resulta

- **Google trae un correo que no tiene cuenta:** se crea, ya confirmada, sin contraseña, con el nombre de Google. **El registro cerrado (`REGISTRATION_ALLOWED_EMAILS`) vale igual:** fuera de la lista no nace ninguna cuenta, tampoco por aquí.
- **El correo ya tiene una cuenta confirmada:** se entra en ella (D77, «misma persona, misma cuenta»). Su contraseña sigue valiendo.
- **El correo tiene una cuenta SIN confirmar** (alguien la registró con contraseña y nunca confirmó el código): puede haberla creado otra persona para quedarse con una puerta. Al entrar con Google esa cuenta queda confirmada **y se le quita la contraseña que tenía** y los códigos pendientes. Quien quiera contraseña la pone después con «olvidé mi contraseña».
- **Cuenta bloqueada:** no entra, como en `/login`.
- **Cuenta con segundo factor activado (D78):** entrar con Google **también lo pide**. `accounts.md` apunta que el de Google lo sustituiría; propongo que no, porque quien activó el segundo factor lo hizo para que una sola llave no baste. **Constrúyelo pidiéndolo** y deja apuntada la duda para el maintainer.
- Se guarda el identificador de Google de la cuenta (`sub`) y, desde entonces, se reconoce por él: si en Google cambia el correo de esa persona, no se enlaza con la cuenta de otro. El correo de la cuenta de Kosmotheka no cambia nunca (D77).
- De Google solo se piden `openid`, `email` y `profile`, y solo se guardan el `sub`, el correo y el nombre. Ningún token de Google se guarda.

## Lo demás

- **Configuración:** `GOOGLE_CLIENT_ID` y `GOOGLE_CLIENT_SECRET`. Vacías (por defecto) = sin Google: las rutas responden que no está disponible y `GET /v1/server` dice `sign_in_with_google: false`. La dirección de vuelta que se registra en Google sale de `DEFAULT_URI`.
- **Límites de peticiones** en `start` y `finish` (por IP), y el de intentos fallidos por cuenta que ya existe.
- **Registro de actividad:** entrar con Google, cuenta creada con Google y cuenta enlazada con Google, con su tipo propio. Sin el `sub` ni nada de Google en la fila.
- **Purga:** los intentos y códigos caducados se borran con la purga horaria.
- **La app:** el botón «Continuar con Google» solo si el servidor lo ofrece; abre el navegador del sistema (el plugin que ya se usa para abrir enlaces), espera la vuelta por el punto de entrada de 7.6.4, y se puede cancelar. Textos en `en` y `es`. Errores con texto propio: cancelado, caducado, este servidor no admite cuentas nuevas, Google no confirma tu correo, cuenta bloqueada.
- **La política de privacidad** (`docs/publicacion/politica-privacidad.md`, en el bloque del borrador para el servidor): qué se recibe de Google.

## Casos hostiles (test antes de la defensa, visto fallar)

1. Un código robado sin el verificador no da token. Con un verificador de otro intento, tampoco.
2. Un código usado dos veces: la segunda no da nada. Un código o un intento caducados, tampoco.
3. **Sesión de otro colada:** Jhonny empieza un intento en su app, entra con SU Google y manda a la víctima el enlace de vuelta `kosmotheka://auth/google?…`. La app de la víctima no lo acepta, porque no es un intento suyo; y si no tiene ninguno pendiente, no llama al servidor.
4. `finish` sin la prueba de dispositivo, o con la de otro dispositivo: no da token.
5. Google dice `email_verified: false`: no se crea ni se enlaza nada.
6. La cuenta sin confirmar con contraseña puesta por otro: tras entrar con Google, esa contraseña ya no entra.
7. La vuelta de Google al servidor con un `state` que no es de ese navegador, o repetido: rechazada.
8. Registro cerrado: un correo de Google fuera de la lista no crea cuenta.
9. Cuenta bloqueada y cuenta con segundo factor: no entran sin lo suyo.
10. Sin `GOOGLE_CLIENT_ID`: ninguna ruta de Google hace nada, y el servidor no lo anuncia.
11. Ni el código, ni el verificador, ni nada de Google aparece en registros, en el registro de actividad ni en mensajes de error; las rutas que llevan el código en la dirección no se pueden evitar del todo (la vuelta de Google): dime cuáles son, para excluirlas del registro de accesos.
12. Dos correos que solo difieren en mayúsculas o espacios son la misma cuenta, con la normalización de siempre.

Google no se puede llamar en los tests: pon su parte detrás de una interfaz y pruébala con un doble, sin red.

## Límites

- Dependencias nuevas: solo las dos de OAuth que D77 ya nombra. Si hace falta otra, **para y dímelo**.
- Doctrine primero; consultas con parámetros; migración con `make migration`.
- Ninguna regla nueva sobre bibliotecas ni permisos: esto solo decide qué cuenta entra. Si algo contradice una decisión aceptada o el encargo crece, **para y dímelo**.
- Modo enseñanza de Rust: las explicaciones, en el informe.
- No deshagas nada con git.

## Al terminar

Todas las comprobaciones (`make check` y `make migration`; `cargo fmt`, `clippy`, `test`; en `app` `build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`) **después del último cambio**, con su salida real. Actualiza `accounts.md`, `security.md` (en llano), `cloud.md` si toca, `errors.md`, `development/server.md` (variables), `servicios-externos.md` y la política de privacidad. Informe a la sesión `library` con SendMessage: el camino tal como quedó, qué decidiste distinto y por qué, tests hostiles y cuáles viste fallar, qué rutas llevan el código en la dirección, y qué no pudiste comprobar (nada de esto se ha probado contra Google de verdad).
