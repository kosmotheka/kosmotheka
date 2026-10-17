# Encargo · `app:user:delete`: borrar una cuenta por consola, y la guía al día

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Aprobado por el maintainer el mismo día. **Por qué:** la política de privacidad promete borrar una cuenta pedida por correo en 30 días, y hoy no hay forma de hacerlo sin SQL a mano. Un `DELETE` en la base se saltaría el traspaso de bibliotecas compartidas (D99). Es una herramienta de administración: **no cambia nada de la API ni de la app.**

## 1. La orden

`app:user:delete <correo>`, en `server/src/Command/`, como las demás `app:user:*` (léelas antes: `UserBlockCommand`, `UserTwoFactorDisableCommand`).

- **Hace exactamente lo mismo que `AccountController::delete`**, en una transacción:
  1. `CloudOwnership::handOverLibrariesOf`;
  2. la línea `ActivityKind::AccountDeleted` del registro de actividad;
  3. `remove` de la cuenta.
- **El mismo código, no una copia.** Saca esas tres líneas a un servicio pequeño (por ejemplo `App\Security\AccountDeletion::delete(User $user, ?string $deviceId)`), que usen el controlador y la orden. En la orden no hay dispositivo: `null`, si `ActivityLog::record` lo admite. Mira cómo lo hacen las otras órdenes que escriben en el registro.
- **Pide confirmación**, como `UserBlockCommand` si lo hace, enseñando qué va a pasar:
  - el correo;
  - cuántas bibliotecas subió;
  - cuáles pasan a otra cuenta y cuáles se borran (lo que diga `CloudOwnership`, si hay forma de preguntárselo sin borrar).

  Con `--force` no pregunta.
- **Un correo que no existe:** error y código de salida distinto de 0, sin decir nada más.
- **El canal del registro:** el que usen las otras órdenes de consola (`console` o el que haya). Si `ActivityKind` o el canal necesitan un caso nuevo, dilo.

## 2. Tests primero

En `server/tests/Command/` (o donde estén las de las otras órdenes):
- **Borrar una cuenta con una biblioteca compartida:** la biblioteca pasa a la otra cuenta (D99), como al borrar desde la app. Escribe el test antes y míralo fallar.
- **Una cuenta sola:** su biblioteca se va con ella.
- **Un correo que no existe:** falla y no toca nada.
- **El registro de actividad:** guarda la línea `AccountDeleted` sin el correo (D66).
- **El controlador** sigue pasando sus tests sin cambiar: es la red del cambio a servicio.

## 3. La guía

- `site/src/content/docs/es/run-your-server/administration.md`:
  - añade la orden a la lista;
  - **rehaz el apartado de borrar la cuenta de alguien que lo pide por correo** con ella: comprobar que quien escribe es el dueño del correo, `app:user:delete`, y contestarle;
  - quita lo que decía que no había forma.
- `es/run-your-server/updating.md`: añade arriba una línea: los pasos se pueden hacer de una vez con `deploy/deploy.sh` (el script del repositorio), con lo que hace y sus opciones. Léelo en `deploy/deploy.sh`.
- `docs/architecture/accounts.md`: una línea en lo de borrar la cuenta.
- `docs/plans/repositorio-publico.md`: en «Lo que falta en el servidor para administrarlo», tacha lo de borrar.

## Al terminar

`make fix` y **un solo `make check`** al final. El informe:
- la orden, con un ejemplo de su salida;
- el servicio común;
- los tests (y que el primero falló antes);
- las páginas cambiadas.
