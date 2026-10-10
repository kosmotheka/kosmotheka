# Encargo 7.7 · Registro de actividad

> Encargo cerrado para la sesión de hitos, 11/10/2026 ([plan](fase-7.md), «7.7 Registro de actividad»). Sin commit, sin push, sin cambiar de rama. Solo `server/` y `docs/`.

## Para qué

Poder decir **quién hizo qué, cuándo y desde dónde** ante una reclamación, aunque lo afectado ya no exista (D66). Lee D66, D52 (accesos del superadministrador y KPI), D84, D98 y `server/AGENTS.md`. Mira cómo están hechos `AccountPurger`, `RequestLimiter`, `EmailFingerprint`, y por dónde entra una operación en `push` hasta aplicarse.

## Qué construir

1. **Una tabla de solo añadir** (`activity_log`, entidad con Doctrine, migración generada con `make migration`):
   - **sin claves foráneas** hacia lo que registra: los ids se copian como valores, para que la fila sobreviva a la cuenta, la biblioteca o el dispositivo borrados;
   - **qué** (un enum cerrado), **cuándo** (hora del servidor, nunca la de la operación ni la del cliente), **quién** (id de la cuenta e id del dispositivo, si los hay), **desde dónde** (IP y agente, este recortado a un largo fijo), **sobre qué** (id de la biblioteca en el servidor y **su nombre en ese momento**, y el id de la persona o dispositivo afectado si aplica);
   - la entidad no tiene modificadores, ni hay ruta, repositorio o comando que cambie o borre una fila salvo la purga por antigüedad.
2. **Qué se registra:**
   - **de la cuenta:** registro confirmado, inicio de sesión (y el fallido contra una cuenta que existe), cierre de sesión de un dispositivo, cambio y restablecimiento de contraseña, activar y quitar el segundo factor, borrar la cuenta, bloqueo por consola;
   - **de las bibliotecas, al aplicarse una operación que llega por `push`:** subir una biblioteca a la nube, borrarla, cambiar un rol, cambiar «Puede compartir», decir de quién es un dispositivo, retirar el acceso a un dispositivo (D71), retirar y devolver el acceso a una persona (D98), encender y apagar el poder de la nube;
   - **de las invitaciones:** crear, anular y aceptar (el tipo, nunca el correo ni el token);
   - **lo que hace la nube sola:** enlazar una cuenta con una persona y vincular un dispositivo.
   Si alguno de estos no existe todavía en el servidor (por ejemplo borrar una biblioteca de la nube), no lo inventes: dímelo en el informe.
3. **El autor de una fila sale del servidor, no del cliente** (D84): la cuenta y el dispositivo son los de la sesión; la biblioteca, la de la tabla de acceso; el nombre, el que el servidor tiene guardado. Lo que diga el cuerpo de la petición solo decide *qué tipo* de evento es, y solo cuando la operación **se ha aplicado** (una rechazada o dejada como petición no deja un «cambió un rol»).
4. **La cuenta borrada** (propuesta mía, a confirmar por el maintainer): al borrar una cuenta, sus filas se quedan con el id de la cuenta y una **huella con clave de su correo** (la de `EmailFingerprint`), no el correo. Permite contestar «¿fue esta dirección?» sin guardar la dirección. El correo en claro no se escribe nunca en esta tabla.
5. **Purga:** al año, en el mismo comando y planificación que `AccountPurger`. El plazo, en una constante o parámetro, no repartido.
6. **Consultarlo, por consola** (el panel es de la Fase 11): un comando `app:activity:list` con filtros por cuenta, biblioteca, tipo y fechas, y un tope de filas. Sin ruta HTTP.
7. **Que el registro no rompa nada ni se pierda en silencio:** decide si la fila va en la misma transacción que lo que registra (lo natural para las operaciones) y qué pasa si falla donde no hay transacción (un inicio de sesión); explícamelo en el informe. Un registro que falla no debe dar acceso ni dejar una acción hecha sin su fila sin que quede rastro en el log de la aplicación.

## Casos hostiles (test antes de la defensa, visto fallar)

1. Una operación con un cambio de rol que el servidor **rechaza** o deja como petición: no hay fila de «cambió un rol».
2. Un cliente que manda en el cuerpo otro id de cuenta, de dispositivo o de biblioteca, u otro nombre de biblioteca: la fila lleva los del servidor.
3. Un agente de usuario de varios megas o con saltos de línea y caracteres de control: se recorta y se limpia; no rompe la petición ni el comando que lista.
4. Una cabecera `X-Forwarded-For` falsa: la IP es la que Symfony considera de confianza según su configuración de proxies; dime qué hay configurado y qué habrá que poner en producción (nginx delante, 7.9).
5. Borrar una biblioteca o una cuenta: sus filas siguen ahí y se listan.
6. Ninguna ruta devuelve filas del registro, ni a quien administra la biblioteca ni a nadie.
7. La purga borra lo de más de un año y nada más reciente.
8. Ni contraseñas, ni códigos, ni tokens, ni correos en claro en ninguna fila: un test recorre las filas creadas por la batería y lo comprueba.

## Reglas

- Doctrine primero; consultas con parámetros, nunca texto concatenado.
- Una sola pieza escribe en la tabla (un servicio `ActivityLog` o como encaje con las convenciones), llamada desde donde ocurre cada cosa; nada de escuchar todas las peticiones a ciegas.
- Sin dependencias nuevas.
- Solo `server/` y `docs/`. Nada en `app/`, `spec/` ni `crates/`. Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. Todo con `cd server && make …`.

## Al terminar

`make check` **después del último cambio**, y `make migration` para comprobar que no queda nada pendiente; dime la salida real. Actualiza `docs/architecture/cloud.md` (o la página del servidor que corresponda), la pieza del registro en `docs/architecture/security.md` en lenguaje llano, el borrador de la política de privacidad si existe en `docs/` (qué se guarda, la IP, un año), `docs/ROADMAP.md` y la línea de 7.7 en `docs/plans/fase-7.md` (sin marcarla). **Manda el informe a la sesión `library` con SendMessage:** qué registras y desde dónde, qué no existe todavía, qué decidiste por tu cuenta, tests hostiles y cuáles viste fallar, comprobaciones, y qué no pudiste comprobar.
