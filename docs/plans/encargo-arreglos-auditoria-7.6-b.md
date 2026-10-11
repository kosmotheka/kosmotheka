# Encargo · Segunda tanda tras la auditoría del 11/10/2026, y el registro cerrado

> Encargo cerrado para la sesión de hitos, 11/10/2026. Sin commit, sin push, sin cambiar de rama. Las tres primeras piezas son hallazgos de [docs/audits/2026-10-11.md](../audits/2026-10-11.md) ya decididos por el maintainer; la cuarta prepara el primer despliegue de prueba. Para cada una: el test del caso primero, visto fallar, y después el cambio.

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

## 4 · Registro cerrado (para el despliegue de prueba; propuesta del coordinador, aceptada por el maintainer como parte del plan del 11/10/2026)

El servidor estará en internet antes de la revisión de seguridad de conjunto: solo deben poder registrarse los correos que el maintainer elija.

- Variable `REGISTRATION_ALLOWED_EMAILS`: lista de correos separados por comas. **Vacía = registro abierto** (lo de hoy, y el valor por defecto para quien monte su servidor). Con contenido, solo esos correos, comparados con la misma normalización que usa el registro.
- Un correo fuera de la lista recibe un error propio, `registration_closed`, **antes** de crear nada ni mandar ningún correo, y sin gastar más que el límite de registro por IP. La app lo enseña con un texto llano en `en` y `es`: «Este servidor no admite cuentas nuevas por ahora».
- `GET /v1/server` dice `registration: "open" | "closed"`, para que la app avise antes de pedir los datos. La lista no se publica nunca.
- Vale para todas las puertas por las que nace una cuenta (hoy, el registro con correo; si Google existiera, también). Quien ya tiene cuenta entra igual. Aceptar una invitación no crea cuentas: no cambia.
- Límite conocido, a apuntar en `cloud.md`: la respuesta distingue si un correo está en la lista. Se acepta para una prueba cerrada.
- Tests hostiles: un correo fuera de la lista no crea cuenta, ni fila, ni correo; mayúsculas y espacios no la esquivan; con la lista vacía todo sigue como hoy; la lista no aparece en ninguna respuesta.
- Documenta la variable donde están las demás (`docs/development/server.md`) y en `security.md`, en llano.

## Límites

- Solo estas cuatro piezas. `server/`, lo mínimo de `app/` para el texto de `registration_closed` y el aviso previo, y `docs/`. Nada en `spec/` ni `crates/`. **No toques `docs/audits/`.**
- Doctrine primero, consultas con parámetros, sin dependencias nuevas. Si algo contradice una decisión aceptada o crece, **para y dímelo**.
- No deshagas nada con git.

## Al terminar

Todas las comprobaciones (`make check` y `make migration` en `server`; `cargo fmt`, `clippy`, `test`; en `app` `build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`) **después del último cambio**, con su salida real. Actualiza `cloud.md`, `security.md`, `accounts.md`, `errors.md`, `politica-privacidad.md` (el registro guarda ahora autor y quien subió) y `development/server.md`. Informe a la sesión `library` con SendMessage: por pieza, qué hiciste, qué test la cubre y si lo viste fallar; qué decidiste por tu cuenta; qué no pudiste comprobar.
