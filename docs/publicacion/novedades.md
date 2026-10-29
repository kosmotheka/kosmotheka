# Novedades de cada versión

> Borradores del texto «Novedades de esta versión» de Google Play (máximo 500 caracteres por idioma). El maintainer elige el número de versión y da el texto por bueno antes de publicar.

## 0.8.0 (29/10/2026)

**Lleva:** la cadena de ramas del 26 al 29/10, la papelera (D105) y el tope de 50 bibliotecas de la nube por cuenta (D106). **Antes de publicarla, desplegar el servidor** con las dos migraciones de la papelera (`Version20261027213609`, `Version20261027220807`); el tope de bibliotecas es configuración (`CLOUD_MAX_LIBRARIES_PER_ACCOUNT`, 50 por defecto), sin migración. Sin el servidor nuevo, una biblioteca en la nube no puede subir lo que se manda a la papelera. **La papelera cambia el formato de los datos:** todos los dispositivos de una biblioteca necesitan esta versión; la línea 2 lo dice.

Es la primera versión con las novedades **condensadas** (como mucho 5 líneas en la app; el detalle completo, en la guía, «Novedades de cada versión»), como se propuso aquí el 29/10/2026 y aceptó el maintainer.

### Las líneas de la app

Español:

1. En el ordenador, la app avisa cuando hay una versión nueva. Para saberlo, pregunta una vez al día a GitHub, que solo ve tu dirección de internet. Puedes apagarlo aquí o en «Acerca de». *(con su interruptor, `check_updates`)*
2. Lo que borras va a la papelera de la biblioteca, en el menú, y se puede restaurar tal como estaba durante 30 días; justo al borrar, «Deshacer» lo devuelve. Todos los dispositivos de la biblioteca necesitan esta versión.
3. En el móvil, el botón atrás ya no pierde lo que escribes: antes de salir de un formulario, pregunta. La autoría, la editorial, la ubicación y las etiquetas se eligen en una hoja con buscador, los géneros con casillas grandes, y el código de barras se escanea también en el formulario.
4. Búsquedas mejores: primero los libros que se llaman como escribes y en tu idioma, con sus ediciones juntas; Open Library trae más autorías en español y Wikidata, más cómics y juegos de mesa. La foto de balda reconoce lo que ya tienes aunque no traiga ISBN.
5. Además: las etiquetas tienen su sitio en el menú; los avisos flotan abajo y se cierran con la X; tocar un ejemplar lo abre para editarlo; las sagas se ordenan arrastrando y avisan si falta el juego base; el historial dice quién hizo cada cambio; y la hoja de cálculo (CSV) lleva todos los tipos.

English:

1. On the computer, the app tells you when a new version is out. To know, it asks GitHub once a day, which only sees your internet address. You can turn it off here or in «About».
2. What you delete goes to the library's trash, in the menu, and can be restored as it was for 30 days; right after deleting, «Undo» brings it back. Every device of the library needs this version.
3. On the phone, the back button no longer loses what you type: it asks before leaving a form. Authors, publisher, location and tags are picked in a sheet with a search box, genres with large checkboxes, and the barcode can also be scanned in the form.
4. Better searches: books called as you typed and in your language come first, with their editions together; Open Library brings more authors in Spanish and Wikidata more comics and board games. The shelf photo knows what you already have even without an ISBN.
5. Also: tags have their own place in the menu; notices float at the bottom and close with the X; tapping a copy opens it to edit; sagas are ordered by dragging and warn when the base game is missing; the history says who made each change; and the spreadsheet (CSV) carries every kind.

**Texto para Google Play** (409 y 388 caracteres). Sin el aviso de versiones, que es solo del ordenador:

- es-ES: Lo que borras va a la papelera de la biblioteca y se restaura durante 30 días; al borrar, «Deshacer» lo devuelve. En el móvil, atrás ya no pierde lo que escribes. Búsquedas mejores: primero los libros que se llaman como escribes y en tu idioma, y más cómics y juegos. Las sagas se ordenan arrastrando y el historial dice quién hizo cada cambio. Todos los dispositivos de una biblioteca necesitan esta versión.
- en-US: What you delete goes to the library's trash and can be restored for 30 days; right after deleting, “Undo” brings it back. On the phone, back no longer loses what you type. Better searches: books called as you typed and in your language first, and more comics and games. Sagas are ordered by dragging and the history says who made each change. Every device of a library needs this version.

## 0.7.0 (preparada el 25/10/2026, para la prueba cerrada)

Solo la app: el servidor no cambia. Lleva también emparejar con un número de 6 cifras (D88): para usarlo, los dos dispositivos necesitan esta versión; el QR sigue valiendo con las anteriores. **Antes de enviarla a revisión**, en Play Console, «Seguridad de los datos»: marcar «Fotos y vídeos › Fotos» (la portada propia llega al servidor con una biblioteca en la nube; la foto de la balda no sale del móvil). **Con ella, desplegar la web**: las páginas de privacidad dicen ya lo de la portada propia y la foto de la balda.

**Texto para Google Play** (480 y 455 caracteres):

- es-ES: Toca la portada en la ficha para poner una foto tuya o buscarla otra vez en los catálogos. Para la primera carga, «Foto de una balda» en «Añadir» lee los lomos y te deja marcar qué añadir. Las series pasan a ser «Sagas y colecciones», y meter una obra en un número ocupado corre las siguientes. El aviso de un catálogo que falla se queda junto a los resultados. Emparejar sin cámara: escribe en el otro dispositivo el número de 6 cifras que sale bajo el QR. El menú, más ordenado.
- en-US: Tap the cover in the form to set a photo of your own or search for it again in the catalogues. For the first load, “Photo of a shelf” in Add reads the spines and lets you tick what to add. Series are now “Sagas and collections”, and putting a work on a taken number moves the following ones up. The warning of a failing catalogue stays next to the results. Pairing without a camera: type on the other device the 6-digit number under the QR. A tidier menu.

## 0.6.0 (25/10/2026, para la prueba cerrada)

Solo la app: el servidor no cambia. **Con ella, desplegar la web**: las páginas de privacidad dicen ya que la cámara también lee el ISBN impreso en una foto, en el móvil. En la ficha de Play, quitar «Por ahora, la nube es por invitación».

**Texto para Google Play** (426 y 398 caracteres):

- es-ES: Si el código de barras de un libro es una etiqueta de la tienda, pulsa «Foto del número» en el escáner: la app lee el ISBN impreso, en el propio móvil. Si reinstalas la app, te ofrece quitar de tu lista las instalaciones anteriores, y cada dispositivo dice cuándo hizo su último cambio. Al entrar con tu cuenta, una pantalla cuenta qué catálogos consulta tu servidor por ti. La Ayuda explica la nube y las personas y permisos.
- en-US: If a book's barcode is a shop label, tap “Photo of the number” in the scanner: the app reads the printed ISBN, on the phone itself. If you reinstall the app, it offers to remove the earlier installs from your list, and each device says when it last changed anything. When you sign in, a screen tells you which catalogues your server asks for you. Help explains the cloud and people and permissions.

## 0.5.0 (24/10/2026, para la prueba cerrada)

**Antes de subirla: el servidor de producción tiene que estar desplegado con este código** (la migración de `cloud_cover` y las rutas de portadas, D82). Sin él, las portadas no pasan por la nube; por la wifi de casa sí, y el resto funciona igual.

**Texto para Google Play** (394 y 406 caracteres):

- es-ES: Las portadas pasan a tus otros dispositivos al sincronizar, por la wifi de casa o por la nube, y ocupan mucho menos: se guardan más pequeñas, también las que ya tenías, y se ven igual de bien. Emparejar con el ordenador es más fácil: el código QR sale más grande y más sencillo. Y al emparejar dos dispositivos que ya tenían bibliotecas, decir de quién es el otro ya no da un error de permisos.
- en-US: Covers now go to your other devices when they sync, over your home Wi-Fi or through the cloud, and take up much less space: they are stored smaller, including the ones you already had, and look just as good. Pairing with a computer is easier: the QR code is bigger and simpler. And when pairing two devices that already had libraries, saying whose the other one is no longer fails with a permissions error.

## 0.4.0 (23/10/2026, para la prueba cerrada)

**Antes de subirla: el servidor de producción tiene que ofrecer GameUPC** (`/v1/server` lo lista en `metadata`). Desplegado y comprobado el 23/10/2026. Sin él, la app no lo pregunta y sigue igual que la 0.3.1.

**Texto para Google Play** (378 y 410 caracteres):

- es-ES: Con cuenta, al escanear un juego de mesa la app pregunta a GameUPC qué juego es, a través del servidor de tu cuenta y sin clave propia. Si duda, eliges el tuyo entre sus propuestas o lo buscas por su nombre; al guardarlo le decimos cuál era para que aprenda (le llega el código, el juego y un identificador de tu cuenta que no dice quién eres). Se apaga en Proveedores de datos.
- en-US: With an account, when you scan a board game the app asks GameUPC which game it is, through your account's server and with no key of your own. If it is not sure, you pick yours among its suggestions or search for it by name; when you save it we tell it which one it was so it learns (it receives the code, the game and an identifier of your account that does not say who you are). Turn it off in Data providers.

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
