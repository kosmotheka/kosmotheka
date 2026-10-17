# Encargo · Guía de usuario, segunda tanda: la cuenta, la nube y las personas

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Igual que la primera tanda ([encargo-guia-usuario-1.md](encargo-guia-usuario-1.md) y su revisión de jerga, [encargo-guia-usuario-1-jerga.md](encargo-guia-usuario-1-jerga.md)): **mismas reglas de escritura y mismas palabras decididas.** Solo `site/src/content/docs/es/using-the-app/` y sus imágenes. Nada de cambiar la app.

## Las páginas

1. **Personas y permisos: administrador, editor, lector y las peticiones.** Qué puede hacer cada uno, pedir en lugar de hacer, aceptar o rechazar, quién borra qué.
2. **Quitar a una persona o un dispositivo.** Echar a alguien (deja de recibir la biblioteca, sigue en el catálogo), devolverle el acceso, quitar un dispositivo perdido y «Dejar este dispositivo a otra persona».
3. **La nube: crear una cuenta, subir una biblioteca e invitar.** Crear la cuenta (correo o Google), la verificación en dos pasos y la huella, subir una biblioteca, el móvil nuevo que recibe sus bibliotecas solo, e invitar con un enlace o por correo.
4. **Fuera de casa y sin conexión: qué funciona.** Material: la Ayuda («away»).
5. **He perdido el móvil o lo he cambiado.** Material: la Ayuda («lost»). Con y sin cuenta, y lo que **no** se puede recuperar sin copia ni cuenta.
6. **Borrar mi cuenta y mis datos.** Desde la app, y la página web `kosmotheka.com/es/borrar-cuenta.html` si no puedes entrar. Que coincida con esa página y con la política de privacidad nueva (`web/es/privacidad.html`).

## Lo que pesa en esta tanda

- **Todo sale de la app tal como es hoy**, comprobado en el código y en `?mock`, no de los planes ni del diseño. Muchas de estas pantallas no las ha visto el maintainer en pantalla todavía. Si el código y el texto de la interfaz no coinciden, o algo te parece un fallo, **apúntamelo en el informe, no lo arregles**.
- **Sin palabras internas:** nada de valla, sello, retirada firmada, tabla de acceso ni operación. Lo que pasa, dicho como pasa: «deja de recibir los cambios», «se borra de su móvil cuando se conecte».
- **Las limitaciones, sin esconderlas.** Sin servidor no hay forma de recuperar una biblioteca que solo estaba en un móvil perdido. Quien administra el servidor puede leer lo que se sube. Los avisos al momento aún no están en el servidor oficial.
- Capturas solo donde ayuden, de `?mock`, con nombres inventados («Ana», «Laura», «Hugo»).

## Al terminar

- `npm run build` en `site/`.
- Para cada página, su título, cuántas palabras tiene y la jerga encontrada y cambiada.
- **La lista de diferencias o posibles fallos** que veas entre lo que la app dice y lo que hace.
