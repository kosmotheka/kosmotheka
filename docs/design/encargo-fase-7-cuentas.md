# Encargo de diseño: la cuenta, la nube e invitar con un enlace (Fase 7)

> Encargo de la Fase 7 para Claude Design, preparado el 02/10/2026. Se sube como guía al proyecto de Design y se pega tal cual lo que va en el bloque. Se basa en D43, D49, D50, D57, D58, D71, D75–D79 y D81 ([DECISIONS.md](../DECISIONS.md)) y en lo que ya hace el servidor ([architecture/accounts.md](../architecture/accounts.md)).

```
Encargo: Fase 7 — La cuenta, la nube e invitar con un enlace (Kosmotheka)

Contexto
Hasta ahora la app funciona sin cuenta: cada dispositivo guarda sus bibliotecas y las comparte por la red local con un QR. En esta fase llega la nube: una cuenta opcional con la que las bibliotecas se sincronizan también a distancia. La nube es "un dispositivo más", siempre encendido, con los mismos roles y peticiones de siempre. Sin cuenta todo sigue igual y debe seguir viéndose como la opción normal, no como una versión recortada.
Partid de vuestras pantallas de las fases 3 a 6 (Ajustes, bibliotecas, personas y roles, dispositivos vinculados, compartir con QR, peticiones y la campanita). Escritorio y móvil, claro y oscuro, textos en español e inglés.
Si algo no está claro, decidid lo más sencillo y listad al final lo que hayáis supuesto.

1. Entrar
- Un sitio discreto pero fácil de encontrar para "Iniciar sesión / Crear cuenta" (Ajustes y, quizá, la primera vez que se abre la app, sin bloquear: "Seguir sin cuenta" debe ser igual de claro).
- Dos formas: email y contraseña, o "Continuar con Google" (abre el navegador del sistema y vuelve a la app). Google solo aparece si el servidor lo ofrece.
- Por defecto se entra en el servidor oficial; un enlace pequeño "Otro servidor" deja escribir la dirección de uno propio.
- Crear cuenta: nombre, email, contraseña (mínimo 8 caracteres) e idioma. Después, una pantalla para teclear el código de 6 cifras que llega por correo (caduca a los 15 minutos, 5 intentos), con "Reenviar código".
- Recuperar la contraseña: email → código → contraseña nueva. Al cambiarla se cierra la sesión en los demás dispositivos de la cuenta; hay que decirlo antes y después.
- Si la cuenta tiene segundo factor: tras la contraseña, pedir el código de la app de autenticación, con "Usar un código de respaldo".

2. La cuenta (Ajustes › Cuenta)
- Nombre, email, idioma; cambiar la contraseña; cerrar sesión.
- Segundo factor (opcional): activarlo con un QR y el secreto para copiar, confirmar con un primer código, y mostrar 10 códigos de respaldo una sola vez (copiar, guardar). Desactivarlo y generar códigos nuevos (piden contraseña y código). Cuántos códigos de respaldo quedan.
- Desbloqueo con huella o cara (solo móvil), también sin cuenta: en Ajustes, apagado por defecto, y la pantalla de bloqueo al abrir la app.
- Mis dispositivos: lista con nombre, último uso y cuál es este. Quitar uno: el aviso dice que, cuando se conecte, borrará sus bibliotecas, y si tiene bibliotecas que no están en la nube, que se perderán. Mientras no se haya conectado se puede deshacer ("Deshacer" en la propia lista).
- Borrar la cuenta (lo pide Google Play): pide la contraseña; avisa de qué pasa con las bibliotecas (las compartidas siguen para los demás; las que solo eran tuyas se borran) y ofrece una copia antes.

3. Dar el dispositivo a otra persona (D81)
- En Ajustes, "Dejar este dispositivo a otra persona": deja la app como recién instalada. Ofrece una copia, avisa de que se borran todas las bibliotecas del dispositivo, y confirma. Existe también sin cuenta.
- Lo que ve quien abre una app que fue quitada desde otro dispositivo: un mensaje claro ("Este dispositivo se quitó de la cuenta de otra persona; sus bibliotecas se han borrado") y la app vacía, lista para usar con o sin cuenta.
- Diferenciad bien "Cerrar sesión" (soy yo y volveré; no se borra nada) de esto.

4. La nube en cada biblioteca
- Al iniciar sesión, la app pregunta qué bibliotecas subir a la nube (solo las que puedes compartir). Al subir una, una casilla marcada: "La nube puede dar acceso a mis dispositivos nuevos" (así un móvil nuevo recibe sus bibliotecas sin vincularlo a mano).
- En la biblioteca, la nube aparece como un sitio más con quien se comparte, junto a los dispositivos de la red local. Subir una biblioteca más tarde, y dejar de tenerla en la nube.
- Un indicador discreto de conectado / sin conexión con la nube, y la campanita con los avisos que ya existen.
- Una persona de la biblioteca puede tener cuenta enlazada o no: que se vea, sin mostrar su email.

5. Invitar con un enlace
- Quien puede compartir crea un enlace para una persona nueva o para una que ya está en la biblioteca, eligiendo su rol. El enlace vale una sola vez y caduca en 24 horas; compartirlo con el menú del sistema (WhatsApp, correo…). El QR de siempre sigue.
- Quien abre el enlace: ve qué biblioteca es y quién le invita, crea la cuenta o inicia sesión, y la biblioteca llega entera.
- Enlace usado o caducado: lo explica y sugiere pedir otro.

6. Errores y estados (para docs/design/errors.md)
Proponed el texto, en español e inglés, de: código incorrecto, código caducado, demasiados intentos (pide otro código), demasiadas peticiones (espera N minutos), email o contraseña incorrectos, cuenta sin confirmar, cuenta bloqueada, código del segundo factor incorrecto, sesión cerrada porque la contraseña cambió en otro dispositivo (no se pierde nada: los cambios sin subir se envían al volver a entrar), dispositivo quitado, sin conexión con la nube, servidor que no responde, invitación caducada o ya usada.
```
