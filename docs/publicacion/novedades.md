# Novedades de cada versión

> Borradores del texto «Novedades de esta versión» de Google Play (máximo 500 caracteres por idioma). El maintainer elige el número de versión y da el texto por bueno antes de publicar.

## Próxima versión (sin número; a elegir por el maintainer)

**Lo que cambia para quien usa la app** (cuentas tras la auditoría del 15/10/2026, D100):

- **Hay que actualizar la app a la vez que el servidor.** La firma con la que un móvil entra en su cuenta ahora dice a qué servidor y para qué es (`kosmotheka-login-v2`), y el servidor **no acepta la forma anterior**: aceptarla dejaría abierto justo lo que se cierra. Una app 0.2.0 no podrá iniciar sesión (ni con contraseña ni con Google) en un servidor con este cambio, y recibirá `invalid_device_proof`. Quien ya tiene sesión abierta no se ve afectado hasta que cierre sesión. **Antes de actualizar el servidor de producción, la versión nueva tiene que estar disponible en la prueba interna.**
- Cambiar la contraseña, restablecerla y borrar la cuenta piden el código de la app de autenticación, si la cuenta lo tiene. Activar el segundo factor pide la contraseña.
- Una cuenta creada con Google ya puede borrarse y activar el segundo factor: pide un código que llega a su correo.
- Al crear la cuenta, la contraseña se elige con el código del correo.
- En un servidor que solo admite algunas direcciones, la app lo avisa antes de registrarse.

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
