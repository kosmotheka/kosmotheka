---
title: Administrar el servidor
description: Quién puede registrarse en tu servidor de Kosmotheka y las órdenes de consola para bloquear cuentas, quitar un segundo factor, leer el registro de actividad y mantener los datos.
sidebar:
  order: 6
---

El servidor no tiene panel web. Se administra con variables de entorno ([la configuración](/es/run-your-server/configuration/)) y con órdenes de consola. Esta página reúne las dos cosas que necesitarás en el día a día.

## Cómo se lanzan las órdenes

Desde el directorio `server/`, con el usuario con que corre el servidor (para que la caché no quede con otro dueño):

| Instalación | Orden |
|---|---|
| Sin Docker | `php8.5 bin/console <orden>` |
| Con Docker | `docker compose run --rm php bin/console <orden>` |

Los ejemplos de abajo usan la forma corta, `bin/console …`. `bin/console list app` enseña las órdenes del proyecto.

## Quién puede registrarse

`REGISTRATION_ALLOWED_EMAILS` es una lista de correos separados por comas, espacios o punto y coma (sin distinguir mayúsculas).

- **Vacía** (por defecto): cualquiera puede crear una cuenta.
- **Con direcciones**: solo esas pueden. Para un servidor familiar, ponla antes de abrirlo a internet.

Qué ve quien **no** está en la lista:

- **Con correo y contraseña**: la app lo trata como cualquier registro, avisa de que el servidor solo admite algunas direcciones y le pide el código de 6 cifras. Ese código **no llega nunca**: el servidor no crea la cuenta ni manda nada. Quien lo intenta no sabe si es la lista o un fallo del correo.
- **Con Google**: ve «Este servidor no admite cuentas nuevas por ahora».

Quien ya tiene cuenta entra como siempre, aunque su correo no esté. Para dar de alta a otra persona, añade su correo a la variable, ejecuta `bin/console cache:clear` y reinicia PHP-FPM (ver [la configuración](/es/run-your-server/configuration/)).

## Las órdenes de consola

### `app:user:block` · bloquear una cuenta

```bash
bin/console app:user:block alguien@ejemplo.org
bin/console app:user:block alguien@ejemplo.org --undo
```

Bloquea la cuenta y **cierra la sesión de todos sus dispositivos**. Con `--undo` la desbloquea. La cuenta bloqueada no puede iniciar sesión y ve «Esta cuenta está bloqueada. Si crees que es un error, escribe a quien gestiona el servidor.» No borra nada: la cuenta, sus bibliotecas y sus datos siguen. Queda en el [registro de actividad](/es/run-your-server/what-the-server-keeps/#el-registro-de-actividad).

### `app:user:delete` · borrar una cuenta

Va más abajo, en [borrar la cuenta de alguien que lo pide por correo](#borrar-la-cuenta-de-alguien-que-lo-pide-por-correo).

### `app:user:two-factor-disable` · quitar el segundo factor

```bash
bin/console app:user:two-factor-disable alguien@ejemplo.org
```

Para quien ha perdido el móvil **y** sus diez códigos de reserva y no puede entrar. Quita el segundo factor de la cuenta; falla si no lo tenía. Después, esa persona entra con su contraseña y puede volver a activarlo.

**Asegúrate de que es quien dice ser antes de ejecutarla**: esta orden es la que convierte «tengo la contraseña» en «entro». Escríbele al correo de la cuenta, o confírmalo por otro medio que conozcas. Queda en el registro de actividad.

### `app:user:super-admin` · el rol de superadministrador

```bash
bin/console app:user:super-admin alguien@ejemplo.org
bin/console app:user:super-admin alguien@ejemplo.org --revoke
```

Concede o retira `ROLE_SUPER_ADMIN`. Solo se puede dar desde la consola, nunca por la API. **Hoy ese rol no abre nada**: el servidor no tiene aún panel ni rutas de administración. Existe para cuando las haya, y no necesitas usarlo.

### `app:activity:list` · leer el registro de actividad

```bash
bin/console app:activity:list --since=2026-10-01 --limit=100
bin/console app:activity:list --account=<id de la cuenta> --kind=login_failed
```

Es la **única** forma de leer el registro (no hay ruta HTTP). Enseña una tabla con las más recientes primero: cuándo (UTC), qué, la cuenta, el dispositivo, quién subió, la biblioteca, el afectado, el detalle, la IP y el agente de usuario.

| Opción | Filtra por |
|---|---|
| `--account` | Una cuenta (su id): lo que hizo o lo que subió su sesión |
| `--library` | Una biblioteca de la nube (su id) |
| `--kind` | Un tipo de suceso, por ejemplo `login_failed`, `password_changed`, `account_blocked`, `invitation_accepted`. Si te equivocas, la orden lista los válidos |
| `--since` · `--until` | Desde y hasta (UTC; `--until` no incluido) |
| `--limit` | Líneas como máximo (50 por defecto, 1000 el tope) |

Las cuentas se identifican por su id, no por el correo: el registro **no guarda el correo**, solo una huella con clave que sirve para comprobar «¿fue esta dirección?» sin tenerla escrita. Qué se apunta se explica en [qué guarda tu servidor](/es/run-your-server/what-the-server-keeps/#el-registro-de-actividad).

### `app:accounts:purge` · la limpieza periódica

```bash
bin/console app:accounts:purge
```

Borra, en una pasada:

- las cuentas **sin confirmar con más de 24 horas**;
- los códigos de correo caducados;
- las invitaciones **caducadas hace más de una semana**;
- las líneas del registro de actividad **de más de un año**;
- los intentos caducados de entrar con Google;
- las búsquedas guardadas que han pasado su plazo (ver [la caché de búsquedas](/es/run-your-server/what-the-server-keeps/#la-caché-de-búsquedas));
- las portadas que ya no usa ninguna obra de su biblioteca, un día después, y las carpetas de portadas de bibliotecas que ya no existen.

**No hace falta lanzarla a mano**: la lanza cada hora el proceso programado (el servicio `worker` de Docker, o `kosmotheka-worker` de systemd, que montaste en la guía de instalación). Sin ese proceso corriendo, no se borra nada. Cada paso va por separado: si uno falla, los demás se hacen igual y la orden termina con un aviso de cuáles fallaron.

### `app:trash:expire` · vaciar lo viejo de la papelera

Lo que alguien borra en la app va a la papelera de su biblioteca y se puede restaurar durante 30 días. Pasado ese plazo lo eliminan los propios dispositivos. Por si en una biblioteca de la nube no se abre ningún dispositivo en mucho tiempo, el servidor elimina lo que lleva **60 días** en la papelera, firmando como el dispositivo de la nube de esa biblioteca. Los 60 días se cuentan desde que la fila llegó al servidor, así que un dispositivo con la hora mal puesta no adelanta nada.

Lo lanza el mismo `worker` de las tareas programadas, una vez al día. Para lanzarlo a mano:

```bash
bin/console app:trash:expire
```

Dice cuántas cosas ha eliminado. Solo actúa en las bibliotecas que tienen su nube con poder; en las demás no hace nada.

### `app:catalog:rebuild` · reconstruir el catálogo

```bash
bin/console app:catalog:rebuild <id de la biblioteca de la nube>
bin/console app:catalog:rebuild --all
```

El servidor guarda de cada biblioteca el **registro de operaciones** (la verdad) y, aparte, un catálogo derivado de él que sirve para consultar. Esta orden borra el catálogo de una biblioteca y lo vuelve a calcular desde el registro, juzgando de nuevo cada operación. Imprime cuántas se aplicaron y cuántas quedaron como peticiones.

Hace falta **poco**: cuando una actualización del servidor cambia cómo se juzgan las operaciones y las notas de la versión lo piden, o si sospechas que el catálogo no cuadra con el registro. Es segura (el registro no se toca) y toma el cerrojo de la biblioteca, así que mientras dura una subida de esa biblioteca espera. Con `--all` reconstruye una por una, cada una en su transacción; si una falla, sigue con las demás y termina con error. El id es el de la biblioteca **de la nube** (el del servidor), el mismo que enseña `app:activity:list --library`.

## Borrar la cuenta de alguien que lo pide por correo

Quien lo pide tiene derecho a que se borre, y la política de privacidad del proyecto promete hacerlo en 30 días como mucho (la tuya dirá lo que tú prometas). Con `app:user:delete` es cosa de un minuto:

1. **Comprueba que quien escribe es el dueño del correo.** La petición tiene que llegar **desde la dirección de la cuenta**. Si llega de otra, pídele que la mande desde la suya, o confírmalo por otro medio que conozcas. Nunca borres una cuenta por una petición que no puedas comprobar: no se deshace.
2. **Si puede entrar en la app**, lo más limpio es que lo haga ella: **Ajustes › Cuenta y nube › Borrar la cuenta**, con su contraseña (o un código por correo si entra con Google) y su segundo factor.
3. **Si no puede entrar, o lo prefieres tú**, lanza la orden con el correo de la cuenta.
4. **Contéstale** cuando esté hecho.

### `app:user:delete` · borrar una cuenta

```bash
bin/console app:user:delete alguien@ejemplo.org
bin/console app:user:delete alguien@ejemplo.org --force
```

Hace **lo mismo que el botón de la app**, con el mismo código: en una sola transacción, las bibliotecas que la cuenta subió a la nube pasan a otra cuenta que tenga un sitio en ellas (primero quien administra, y entre iguales la más antigua), o se borran con ella si no queda nadie; se apunta una línea en el registro de actividad y se borra la cuenta, con sus dispositivos y sus códigos.

Sin `--force`, antes de borrar **enseña qué va a pasar y pide confirmación** (por defecto, «no»):

```
Account: alguien@ejemplo.org (0196b2f1-…)
It uploaded 2 cloud libraries:
  - 0196b2f3-…: passes to account 0196b2f0-…
  - 0196b2f4-…: is deleted with the account (nobody else has a place in it)
Delete this account? It cannot be undone. (yes/no) [no]:
```

Con `--force` no pregunta, para un guion. Si el correo no tiene cuenta, falla con un mensaje de error y salida distinta de cero, sin tocar nada. En el registro de actividad queda **una línea `account_deleted`** con el id de la cuenta y la huella de su correo, **sin el correo**, y con el canal `console`; se conserva hasta cumplir su año.

**No borres las filas de la base de datos a mano**: un `DELETE` directo se salta el traspaso de bibliotecas y puede llevarse la biblioteca de todos.
