# Encargo 7.6.1 · Invitaciones: el servidor

> Encargo cerrado para la sesión de hitos, 10/10/2026. Primera de las tres partes de 7.6 ([plan](fase-7.md), «7.6 Invitar con un enlace»). Sin commit, sin push, sin cambiar de rama.

## Qué se decide y dónde está escrito

Lee antes, enteros: `AGENTS.md` y `server/AGENTS.md`; **D43 con su enmienda del 10/10/2026** en `docs/DECISIONS.md` (es la especificación); D76, D79, D84, D96 y D98; `docs/architecture/cloud.md` (`claim`, `AccountLinker`, `AccessCutter`, avisos) y `docs/architecture/accounts.md` (correo, límites de peticiones, códigos); `docs/architecture/security.md`. En `docs/plans/fase-7.md`, la sección 7.6 y sus **diez casos hostiles**.

Resumen de la regla, por si algo no cuadra con D43 (manda D43):

- Invita **quien administra** una biblioteca que está en la nube, que **tiene personas y roles** y donde **la nube puede vincular dispositivos** (`library.cloud_device_id` con valor y de confianza).
- Una invitación es de **un solo uso**, caduca a las **24 horas** y quien administra puede **anularla**. Del token solo se guarda la huella.
- **Dos tipos:**
  - `email`: atada a un correo. Solo la acepta una cuenta cuyo correo verificado es ese. Sirve para una persona nueva (lectora) o para una que ya está en la biblioteca, que **conserva su rol**. El servidor manda el correo con el enlace. Se guarda una huella del correo normalizado, no el correo.
  - `link`: para copiar. Solo para una **persona nueva y lectora**. No se manda nada: el servidor devuelve el enlace una vez.
- **Nunca** para una persona que administra, que tiene el acceso retirado, o que ya tiene una cuenta enlazada.
- Aceptar enlaza la cuenta con esa persona: es la segunda forma de hacerlo, junto a la del dispositivo con clave avalada (D96). El dispositivo de quien acepta se vincula después por el camino que ya existe (`claim`, `device.link_account`).

## Qué construir (solo `server/` y `docs/`)

1. **Entidad `Invitation`** (Doctrine, migración): biblioteca de la nube, id de la persona (`member`), tipo, huella del token, huella del correo (solo `email`), cuenta que la creó, creada, caduca, aceptada (cuándo y por qué cuenta), anulada. Nada que impida más adelante una invitación de varios usos o sin caducidad (por ejemplo, no pongas «un solo uso» como restricción de la tabla, sino como regla del código), pero **no la construyas**.
2. **Rutas** (bajo `/v1/cloud/…`, con el estilo de las que hay; propón los nombres):
   - crear: recibe la biblioteca de la nube, el id de la persona, el tipo y, si es `email`, el correo. La operación que crea a la persona nueva la firma y la sube antes el dispositivo de quien administra: el servidor **no crea personas**. El id de la persona es un id del cliente (D84): se **compara** con el catálogo de **esa** biblioteca de la nube, nunca se usa para decidir a qué biblioteca pertenece;
   - listar las pendientes de una biblioteca (solo quien administra) y anular una;
   - mirar una invitación con su token, **sin sesión**: lo justo para que la app enseñe «Te invitan a «Casa»» o por qué no vale (caducada, usada, anulada). Nada más: ni personas, ni roles, ni quién invita con su correo. Decide si el nombre de quien invita se enseña y dímelo;
   - aceptar, **con sesión**: comprueba todo otra vez (ver abajo), enlaza la cuenta con la persona y da acceso (`Membership`), en una transacción con bloqueo, de modo que dos aceptaciones a la vez den una sola.
3. **Quién administra** se lee del catálogo en el servidor (la persona de la cuenta, o de su dispositivo, con rol `admin` y con acceso), no de nada que diga el cliente. Mira cómo lo hacen `CloudLibraryAccess`, `AccessCutter` y `Permissions`; no dupliques la regla.
4. **Al aceptar se vuelve a comprobar:** la invitación (existe, no caducada, no usada, no anulada); que quien la creó **sigue administrando** y tiene acceso; que la persona existe en esa biblioteca, no administra, no tiene el acceso retirado y no tiene cuenta; para `link`, que es lectora; para `email`, que el correo de la cuenta coincide; que la cuenta **no es ya otra persona** en esa biblioteca; que la biblioteca sigue teniendo roles y la nube el poder de vincular.
5. **El enlace:** `<base>/i/<token>` con la base en una variable de entorno (D79: `kosmotheka.app` en producción). Token aleatorio de al menos 256 bits. Mira si D79 permite llevar el token en el fragmento (`#`) para que no quede en registros de servidores web; si no está claro, déjalo en la ruta y dímelo.
6. **El correo de invitación:** con el mailer que ya hay (Mailpit en desarrollo). Texto fijo, en el idioma de la cuenta que invita si se conoce; lo único que viene de quien invita es el **nombre de la biblioteca**, escapado. Sin nombre ni correo de quien invita en el asunto.
7. **Límites de peticiones:** crear invitaciones (por cuenta), invitaciones por correo por cuenta y día (bajo: propón la cifra), mirar y aceptar (por IP y por cuenta). Con los limitadores que ya usa el servidor.
8. **Aviso a quien administra** cuando alguien acepta: por el canal de avisos al momento que ya existe (tema de la biblioteca o de la cuenta; mira `SyncNotifier` y `NoticeTopic`). El aviso no lleva datos personales: dice que hay una invitación aceptada en esa biblioteca; la app pregunta lo demás.
9. **Purga:** las invitaciones caducadas o usadas se borran pasado un tiempo, con la purga horaria que ya existe.
10. **Códigos de error** propios y estables (`invitation_expired`, `invitation_used`, `invitation_revoked`, `invitation_not_for_this_account`…), documentados. A quien no tiene derecho a saber si una biblioteca existe se le responde lo mismo que hoy.

## Seguridad: lo que no se negocia

- **Los diez casos hostiles del plan, cada uno con su test escrito antes de la defensa y visto fallar.** Dime en el informe cuáles viste fallar y cuáles no.
- Todo acceso a la base de datos con Doctrine y parámetros enlazados; ninguna consulta montada con cadenas.
- El servidor nunca se fía de un id del cliente (D84).
- La comparación de huellas, en tiempo constante. La huella del token no necesita sal (el token es aleatorio); la del correo sí necesita un secreto del servidor (un correo se adivina): usa uno de entorno, sin valor en el repositorio.
- Ningún secreto en el repositorio ni en los tests.

## Límites

- Solo `server/` y `docs/`. Nada en `app/`, `crates/` ni `spec/`. Si crees que hace falta una operación o una regla nueva en `spec/`, **para y dímelo**.
- El servidor no gana ningún poder nuevo dentro de la biblioteca: no crea personas ni cambia roles.
- Si algo contradice una decisión aceptada, o el encargo crece, para y explícamelo en vez de decidir.
- No deshagas nada con git. Herramientas por `cd server && make …` (Docker).

## Al terminar

`make check` entero en verde y sin migración pendiente (`make migration` debe decir que no hay cambios). Documenta en `docs/architecture/cloud.md` (rutas, reglas, códigos, límites, la variable de entorno nueva y lo que necesita quien aloja su servidor) y actualiza `docs/architecture/security.md` en lenguaje llano. Línea de 7.6.1 en «Estado» de `docs/plans/fase-7.md`, sin marcarla. Informe corto: rutas y nombres elegidos, qué decidiste por tu cuenta, tests por caso hostil y cuáles viste fallar, resultado de `make check`, y qué no pudiste comprobar.
