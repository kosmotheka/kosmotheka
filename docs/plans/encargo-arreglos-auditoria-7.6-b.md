# Encargo · Segunda tanda tras la auditoría del 11/10/2026

> Encargo cerrado para la sesión de hitos, 11/10/2026. Sin commit, sin push, sin cambiar de rama. Las tres primeras piezas son hallazgos de [docs/audits/2026-10-11.md](../audits/2026-10-11.md) ya decididos por el maintainer. Para cada una: el test del caso primero, visto fallar, y después el cambio.

## 1 · SEG-041 · El registro apunta quién lo hizo y quién lo subió (D66, enmienda del 11/10/2026)

Hoy una fila que sale de una operación lleva la cuenta, el dispositivo y la IP de **quien la sube**. Entre dispositivos de una casa es normal que uno suba operaciones de otro.

- La fila gana el **autor**: el dispositivo que firmó la operación (el que el servidor ya verificó con su clave, no un texto del cuerpo sin comprobar) y la cuenta de **su persona** en esa biblioteca, leída de la tabla de acceso del servidor; nula si esa persona no tiene cuenta. Y conserva, con nombre que no confunda, **quien la subió**: cuenta, dispositivo, IP y agente de la sesión.
- Para lo que firma la nube, el autor es la nube y «para quién» sigue como está.
- Migración generada con `make migration`. `app:activity:list` enseña los dos y filtra por cualquiera de las dos cuentas.
- Tests hostiles: una operación de Ana subida por el dispositivo de Bea queda a nombre de Ana y «subida por» Bea, con la IP de Bea; un dispositivo que sube una operación cuyo autor no es de ninguna cuenta; el cuerpo no puede elegir el autor (es el de la firma verificada).

## 2 · SEG-042 · El correo de invitación no lleva el nombre de la biblioteca (D43, enmienda del 11/10/2026)

- El correo dice «Te han invitado a una biblioteca de Kosmotheka» (en `en` y `es`, como los demás correos) y no lleva **nada** escrito por quien invita: ni nombre de biblioteca, ni de persona. El nombre se ve en la app al abrir el enlace, como ya ocurre.
- Quita el filtro del nombre si ya no se usa en ningún otro correo. Test: una biblioteca con un dominio en el nombre no deja rastro de él en el correo (asunto, texto y HTML).

## 3 · SEG-044 · El tope de correos por destinatario (decidido el 11/10/2026)

- **3 al día por pareja (biblioteca, dirección)** y **10 al día por dirección** en total. El de 10 por cuenta se queda.
- Se comprueban los tres **antes** de gastar ninguno, y **no se gasta ninguno si el correo no se envía** (falla la validación, la regla o el envío).
- Tests hostiles: Jhonny manda 3 invitaciones a Laura desde su biblioteca y la familia aún puede invitarla; al llegar a 10 en total, nadie más ese día; un envío que falla no gasta cuota.
- `laura+1@…` frente a `laura@…`: **no lo normalices** (cada proveedor trata los alias a su manera); déjalo apuntado en `cloud.md` como límite conocido.

## 4 · Registro cerrado: **no es tuyo**

Lo construye la sesión coordinadora en paralelo (registro con lista de correos permitidos: `REGISTRATION_ALLOWED_EMAILS`, error `registration_closed`, `registration` en `GET /v1/server`). **No toques** `AccountController::register` y sus tests, `ServerController`, ni las claves de traducción de la cuenta; si necesitas tocar un fichero que huela a eso, para y dímelo. En `en.json`, `es.json`, `cloud.md` y `security.md` toca solo tus secciones.

## Límites

- Solo las piezas 1 a 3. `server/` y `docs/`; nada en `app/`. Nada en `spec/` ni `crates/`. **No toques `docs/audits/`.**
- Doctrine primero, consultas con parámetros, sin dependencias nuevas. Si algo contradice una decisión aceptada o crece, **para y dímelo**.
- No deshagas nada con git.

## Al terminar

Todas las comprobaciones (`make check` y `make migration` en `server`; `cargo fmt`, `clippy`, `test`; en `app` `build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`) **después del último cambio**, con su salida real. Actualiza `cloud.md`, `security.md`, `accounts.md`, `errors.md`, `politica-privacidad.md` (el registro guarda ahora autor y quien subió) y `development/server.md`. Informe a la sesión `library` con SendMessage: por pieza, qué hiciste, qué test la cubre y si lo viste fallar; qué decidiste por tu cuenta; qué no pudiste comprobar.
