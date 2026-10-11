# Novedades de cada versión

> Borradores del texto «Novedades de esta versión» de Google Play (máximo 500 caracteres por idioma). El maintainer elige el número de versión y da el texto por bueno antes de publicar.

## Siguiente a la 0.1.6 (borrador del 11/10/2026)

**Lo que cambia para quien usa la app** (60 commits desde la 0.1.6, del 05/10/2026):

- **Cuenta y nube, opcionales:** crear una cuenta y subir una biblioteca para tenerla en todos tus dispositivos sin que estén en la misma wifi. Sin cuenta, todo sigue igual que antes.
- **Invitar a una biblioteca**, por correo o con un enlace.
- **Quitar a una persona de una biblioteca** y devolverle el acceso.
- **El móvil nuevo de una persona** recibe sus bibliotecas al iniciar sesión.
- Arreglo: en el móvil, el toque que abría el formulario de editar a veces lo enviaba.

**A decidir antes de publicar:** esta versión enseña la cuenta y la nube a todo el que la instale, pero el servidor solo admite por ahora los correos de la prueba (`REGISTRATION_ALLOWED_EMAILS`); los demás verán «Este servidor no admite cuentas nuevas por ahora». Las notas de abajo lo dicen. La alternativa es esperar a abrir el registro.

### Español (383 caracteres)

```
Llega la nube, de momento en pruebas y por invitación: con una cuenta podrás tener tu biblioteca en todos tus dispositivos sin compartir wifi, e invitar a otras personas por correo o con un enlace. Es opcional: sin cuenta, la app funciona igual que siempre, y tus datos siguen en tu dispositivo.

También: quitar a una persona de una biblioteca y un arreglo al editar desde el móvil.
```

### English (368 characters)

```
The cloud arrives, for now in testing and by invitation: with an account you will be able to keep your library on all your devices without sharing wifi, and invite other people by email or with a link. It is optional: with no account the app works as always and your data stays on your device.

Also: removing a person from a library, and a fix when editing on phones.
```

### Antes de publicar

- La [política de privacidad](politica-privacidad.md) publicada tiene que contar ya lo de la cuenta y el servidor (hoy dice «sin cuentas ni servidores»): se cambia a la vez en `politica-privacidad.md` y en las dos páginas de la web. **Google Play lo comprueba**, y el formulario «Seguridad de los datos» de Play Console también cambia (correo, nombre, datos de la biblioteca en un servidor).
- Borrar la cuenta desde la app: Google Play lo exige en cuanto hay cuentas. Comprobar que está y que funciona contra el servidor real.
