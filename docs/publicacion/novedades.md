# Novedades de cada versión

> Borradores del texto «Novedades de esta versión» de Google Play (máximo 500 caracteres por idioma). El maintainer elige el número de versión y da el texto por bueno antes de publicar.

## 0.3.0 (18/10/2026, para la prueba interna y después la cerrada)

**Antes de subirla: el servidor de producción tiene que estar desplegado con este código.** «Salir de la biblioteca» es una regla nueva que el servidor tiene que conocer (D102); un servidor anterior la convertiría en una petición a quien administra.

**Texto para Google Play** (445 y 409 caracteres):

- es-ES: Dos formas de irte de una biblioteca: «Quitar de este dispositivo» (sigue en tu nube y puedes volver a bajarla) y «Salir de la biblioteca» (dejas de tener acceso en todos tus dispositivos; tus libros siguen en ella). «Borrar biblioteca» solo para quien administra. Las series cuentan igual en la lista y dentro. Nombres más claros para quitar un dispositivo. La Ayuda enlaza con la guía completa. En el ordenador, «Escanear» pasa a «¿Lo tengo?».
- en-US: Two ways to go from a library: "Remove from this device" (it stays in your cloud and you can bring it down again) and "Leave the library" (you lose access on all your devices; your books stay in it). "Delete library" only for those who administer. Series count the same in the list and inside. Clearer names for removing a device. Help links to the full guide. On the computer, "Scan" becomes "Do I have it?".

## 0.2.1 (17/10/2026, para la prueba interna)

**Texto para Google Play** (403 y 389 caracteres):

- es-ES: Ahora la app te pide tu nombre y al emparejar dices de quién es cada dispositivo y su rol. Las acciones delicadas de la cuenta piden el código de verificación si lo tienes. Al añadir, un ISBN con guiones o de 10 cifras se reconoce. Con cuenta, juegos de mesa y libros de rol desde BoardGameGeek. Importante: borra los datos de prueba antes de usarla («Dejar este dispositivo a otra persona» en Ajustes).
- en-US: The app now asks for your name, and when pairing you say whose each device is and its role. Sensitive account actions ask for your verification code if you use one. Adding recognises an ISBN with hyphens or ten digits. With an account, board games and role-playing books from BoardGameGeek. Important: delete your test data before using it ("Hand this device to someone else" in Settings).


> **Las novedades que ve la gente dentro de la app salen de `app/src/whats-new.json`** (las líneas de «next»). Este documento es el borrador del texto de Google Play: sale de ese fichero, no al revés. Al elegir el número de versión, renombra `next` por él en el fichero (si no, `npm run check:whats-new` falla) y copia aquí lo que valga para Play (500 caracteres por idioma).

**Lo que cambia para quien usa la app** (cuentas tras la auditoría del 15/10/2026, D100):

- **Hay que actualizar la app a la vez que el servidor.** La firma con la que un móvil entra en su cuenta ahora dice a qué servidor y para qué es (`kosmotheka-login-v2`), y el servidor **no acepta la forma anterior**: aceptarla dejaría abierto justo lo que se cierra. Una app 0.2.0 no podrá iniciar sesión (ni con contraseña ni con Google) en un servidor con este cambio, y recibirá `invalid_device_proof`. Quien ya tiene sesión abierta no se ve afectado hasta que cierre sesión. **Antes de actualizar el servidor de producción, la versión nueva tiene que estar disponible en la prueba interna.**
- Cambiar la contraseña, restablecerla y borrar la cuenta piden el código de la app de autenticación, si la cuenta lo tiene. Activar el segundo factor pide la contraseña.
- Una cuenta creada con Google ya puede borrarse y activar el segundo factor: pide un código que llega a su correo.
- Al crear la cuenta, la contraseña se elige con el código del correo.
- En un servidor que solo admite algunas direcciones, la app lo avisa antes de registrarse.

**Lo que cambia para quien usa la app** (roles obligatorios, D97):

- **La primera vez, la app pide tu nombre.** Tu biblioteca se crea a tu nombre, como administrador o administradora; sin nombre no hay biblioteca. Se cambia luego en Ajustes (vale para las bibliotecas que crees después).
- **Al emparejar un dispositivo hay que decir de quién es y con qué rol** («editor» por defecto; dar «administrador» lleva un aviso).
- Ya no existe una biblioteca «sin roles»: desaparecen la pantalla para activarlos y las preguntas de «quién eres».
- **Restaurar una copia en un móvil nuevo no une la biblioteca con la original**: queda como una biblioteca nueva con los mismos libros. Para tener la original, empareja con un dispositivo que la tenga o bájala de la nube.

**Hay que borrar los datos de prueba antes de usar esta versión** (las bibliotecas de antes no se convierten): en cada dispositivo, «Dejar este dispositivo a otra persona» en Ajustes; y en la nube, borrar la cuenta y volver a registrarse. El servidor también cambia (migraciones nuevas, el sello desaparece), así que va a la vez que la app.

**Antes de publicar:** comprobar que `DEFAULT_URI` del servidor de producción es exactamente `https://api.kosmotheka.app` (es parte de lo que se firma; ver [development/server.md](../development/server.md)), y que la app de la prueba interna ya está actualizada.

## 0.2.0 (11/10/2026, subida a la prueba interna)

**Lo que cambia para quien usa la app** (60 commits desde la 0.1.6, del 05/10/2026):

- **Cuenta y nube, opcionales:** crear una cuenta y subir una biblioteca para tenerla en todos tus dispositivos sin que estén en la misma wifi. Sin cuenta, todo sigue igual que antes.
- **Invitar a una biblioteca**, por correo o con un enlace.
- **Continuar con Google** para crear la cuenta o entrar.
- **Buscar un libro por ISBN a través del servidor**, para quien tiene cuenta (Google Books sin clave falla a menudo).
- **Quitar a una persona de una biblioteca** y devolverle el acceso.
- **El móvil nuevo de una persona** recibe sus bibliotecas al iniciar sesión.
- Arreglo: en el móvil, el toque que abría el formulario de editar a veces lo enviaba.

**A decidir antes de publicar:** esta versión enseña la cuenta y la nube a todo el que la instale, pero el servidor solo admite por ahora los correos de la prueba (`REGISTRATION_ALLOWED_EMAILS`); los demás verán «Este servidor no admite cuentas nuevas por ahora». Las notas de abajo lo dicen. La alternativa es esperar a abrir el registro.

### Español (464 caracteres)

```
Llega la nube, de momento en pruebas y por invitación: con una cuenta (con correo o con Google) tendrás tu biblioteca en todos tus dispositivos sin compartir wifi, y podrás invitar a otras personas por correo o con un enlace. Es opcional: sin cuenta, la app funciona igual que siempre y tus datos siguen en tu dispositivo.

También: con cuenta, buscar un libro por su ISBN falla menos; quitar a una persona de una biblioteca; y un arreglo al editar desde el móvil.
```

### English (435 characters)

```
The cloud arrives, for now in testing and by invitation: with an account (email or Google) you can keep your library on all your devices without sharing wifi, and invite other people by email or with a link. It is optional: with no account the app works as always and your data stays on your device.

Also: with an account, looking a book up by ISBN fails less often; removing a person from a library; and a fix when editing on phones.
```

### Antes de publicar

- La [política de privacidad](politica-privacidad.md) publicada tiene que contar ya lo de la cuenta y el servidor (hoy dice «sin cuentas ni servidores»): se cambia a la vez en `politica-privacidad.md` y en las dos páginas de la web. **Google Play lo comprueba**, y el formulario «Seguridad de los datos» de Play Console también cambia (correo, nombre, datos de la biblioteca en un servidor).
- Borrar la cuenta desde la app: Google Play lo exige en cuanto hay cuentas. Comprobar que está y que funciona contra el servidor real.
