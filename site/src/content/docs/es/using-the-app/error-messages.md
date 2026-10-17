---
title: Mensajes de error y qué hacer
description: "Los avisos que puede enseñar Kosmotheka cuando algo no sale, qué significan y qué hacer en cada caso."
sidebar:
  order: 20
---

Casi todos los avisos de la app se arreglan en un minuto. Aquí están los que de verdad te puedes encontrar, escritos tal como salen en pantalla, con lo que significan y lo que puedes hacer. Si buscas uno, usa la búsqueda de tu navegador (Ctrl+F) con las primeras palabras.

Una regla que ayuda: **la app funciona sin internet y sin cuenta**. Casi ningún aviso de esta página impide usarla; solo te dicen que algo, como buscar datos o sincronizar, no ha podido hacerse ahora.

## Sin conexión

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «La búsqueda ha fallado. Revisa la conexión.» | La app no ha podido preguntar a ningún catálogo: no hay internet o el servicio no contesta. | Mira si tienes internet y prueba de nuevo. Mientras, rellena la ficha a mano: solo el título es obligatorio. |
| «No se pudo consultar *Servicio*.» | Ese catálogo en concreto no ha respondido. Los demás sí. | Normalmente es pasajero. Inténtalo en unos minutos. Si el resultado que buscas aparece por otro catálogo, no hace falta hacer nada. |
| «No está en los catálogos online. Puedes rellenarlo a mano.» | Ningún catálogo conoce ese libro, disco o juego. Pasa con ediciones raras o muy antiguas. | Rellénalo a mano. No es un fallo. |
| «Sin conexión con la nube» | La app no llega al servidor de tu cuenta. | Todo sigue funcionando en tu dispositivo y por la wifi de casa. Los cambios se enviarán solos cuando vuelva la conexión. |
| «Sin conexión en directo. Los cambios de la nube llegan al sincronizar» | El servidor no avisa al momento (o no hay conexión). | Nada que hacer: los cambios llegarán la próxima vez que la app sincronice. Si quieres, ábrela de vez en cuando. |
| «Este servidor no envía avisos al momento.» | Tu servidor no tiene los avisos instantáneos. | Nada que hacer: los cambios llegan cuando la app se pone al día. |

## El servidor no responde

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «El servidor no responde. Puede estar en mantenimiento, o la dirección no es correcta.» | La app no consigue hablar con el servidor. | Pulsa **Reintentar** en unos minutos. Si usas un servidor propio, comprueba que la dirección está bien escrita. Mientras, la app sigue funcionando. |
| «Esta dirección no es válida. Comprueba que sea la de un servidor Kosmotheka.» | La dirección del servidor está mal escrita, o no es una que la app admita. | Copia la dirección completa, con `https://` y sin nada detrás. |
| «Esta versión de la app no puede sincronizar con el servidor.» | La app y el servidor son de versiones muy distintas. | Actualiza la app e inténtalo de nuevo. |
| «La app y el servidor tienen versiones distintas. Actualiza la app.» | Lo mismo, al subir o bajar una biblioteca. | Actualiza la app. |
| «Algo ha fallado con la nube. Prueba otra vez.» | Un fallo general de la nube. | Vuelve a intentarlo. Si se repite, avisa a quien lleva el servidor. |
| «Algo ha fallado.» (en las pantallas de cuenta) | El servidor ha dicho que no a algo que no esperaba. | Inténtalo de nuevo. Si se repite, avisa. |
| «Demasiadas peticiones. Espera *n* minutos e inténtalo de nuevo.» | Has intentado muchas veces seguidas y el servidor te hace esperar. | Espera lo que dice y vuelve a probar. |

## La cuenta

### Entrar y crear la cuenta

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «El email o la contraseña no son correctos.» | Alguno de los dos no coincide. | Revisa cómo está escrito el email. Si no recuerdas la contraseña, pulsa **Recuperar la contraseña**. |
| «Falta confirmar tu email.» | Creaste la cuenta pero no escribiste el código que te llegó. | Pulsa **Enviar otro código** y escríbelo. Las cuentas que no se confirman se borran al día siguiente. |
| «Código incorrecto.» | El código no es el que te mandamos. | Usa el del último correo que te ha llegado: si pediste varios, solo vale el último. |
| «Este código ha caducado.» | Los códigos valen 15 minutos. | Pide otro con **Enviar otro código** y úsalo en cuanto llegue. |
| «Demasiados intentos con este código.» | Fallaste varias veces y el código ya no sirve. | Pide uno nuevo con **Pedir otro código**. |
| «Este servidor no admite cuentas nuevas por ahora.» | El servidor solo deja crear cuenta a algunas personas. | Si ya tienes cuenta, inicia sesión. Si esperabas poder crear una, habla con quien lleva el servidor. |
| «La contraseña necesita al menos 8 caracteres.» | Es demasiado corta. | Elige una más larga. |
| «Esta cuenta está bloqueada.» | Quien lleva el servidor la ha bloqueado. | Escríbele. Si crees que es un error, él lo puede arreglar. |
| «Este dispositivo tiene la sesión iniciada en otra cuenta.» | Este móvil u ordenador ya está en uso con otra cuenta. | Para usarlo con la tuya, ve a **Ajustes › Este dispositivo** y déjalo a otra persona: queda como recién instalado. Ojo: borra lo que hubiera en él (haz antes una copia). |
| «Tu cuenta conoce este dispositivo con otra clave.» | La cuenta recuerda este dispositivo de antes, de otra instalación de la app. | Quítalo de tu cuenta desde otro dispositivo, en **Cuenta › Mis dispositivos**, y vuelve a entrar aquí. |

### Entrar con Google

| Lo que ves | Qué hacer |
|---|---|
| «Has cancelado el inicio de sesión con Google.» | Nada. Si fue sin querer, empieza de nuevo. |
| «El inicio de sesión con Google ha tardado demasiado.» | Empieza de nuevo desde la app. |
| «Google no ha confirmado tu correo.» | Confirma la dirección en tu cuenta de Google, o entra con tu correo y contraseña. |
| «Este correo pertenece a otra cuenta de Google.» | Entra con la cuenta de Google que usaste antes, o con tu correo y contraseña. |
| «No ha funcionado entrar con Google.» | Inténtalo en un momento, o entra con tu correo y contraseña. |

### El segundo factor (código de la app de autenticación)

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «Esta cuenta pide el código de su app.» | Tienes activado el segundo factor. | Escribe el código de 6 cifras que enseña tu app de autenticación. |
| «El código no es correcto.» | El código cambia cada 30 segundos y el que escribiste ya no vale, o la hora del móvil no es la buena. | Escribe el que se ve ahora. Comprueba que la hora del móvil sea automática. Si no tienes la app, usa un **código de respaldo**. |
| «Ese código de respaldo no vale.» | Cada código de respaldo vale una sola vez. | Prueba con otro de tu lista. |
| «El segundo factor ya estaba activado.» / «El segundo factor no está activado.» | Lo que ves en pantalla no está al día. | Vuelve a abrir la pantalla de la cuenta. |
| «No hay ninguna activación en curso.» | Se cerró la pantalla a medias. | Vuelve a pulsar **Activar**. |

Si pierdes el móvil **y** tus códigos de respaldo, mira [He perdido el móvil o lo he cambiado](../lost-phone/): quien lleva el servidor puede quitarte el segundo factor.

### Cuando se cierra la sesión

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «Se ha cerrado la sesión. La contraseña se cambió en otro dispositivo.» | Cambiaste la contraseña en otro sitio y por seguridad se cerraron las demás sesiones. | Entra con la contraseña nueva. No se pierde nada: lo que no se había subido se envía al volver a entrar. |
| «Este dispositivo se quitó de una cuenta.» | Alguien, desde otro dispositivo de la cuenta, lo quitó. Sus bibliotecas se han borrado de aquí. | La app queda vacía y lista para usar, con o sin cuenta. Si era tu cuenta, entra otra vez y lo que tenías en la nube volverá. |
| «Vuelve a entrar para continuar.» | La sesión no estaba abierta. | Pulsa **Iniciar sesión**. |

## Compartir y emparejar

### Emparejar dos dispositivos por la wifi

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «Eso no es un código de emparejamiento.» | Lo que has escaneado o pegado no es el código. | Escanea el QR o copia el código entero. |
| «Ese código es de este mismo dispositivo.» | Estás leyendo el código en el mismo aparato que lo muestra. | El código se enseña en un dispositivo y se lee en **otro**. |
| «No se encuentra el otro dispositivo. ¿Está en la misma wifi y con la app abierta?» | Los dos no se ven. | Comprueba que están en la misma wifi (no con datos móviles ni en la red de invitados) y que la app está abierta y con el código a la vista. Una VPN o el cortafuegos también pueden estorbar. |
| «El otro dispositivo lo ha rechazado: el código caducó o ya se usó, o ha olvidado este dispositivo.» | Los códigos valen una vez y poco tiempo. | En el otro dispositivo, genera un código nuevo. Si lo había olvidado, empareja de nuevo. |
| «El dispositivo que respondió no es el del código. Se ha detenido el emparejamiento por seguridad.» | Otro aparato de la red ha contestado en lugar del que esperabas. | Es una protección. Comprueba que estás en tu red, genera un código nuevo e inténtalo otra vez. |
| «El otro dispositivo ya tiene emparejado este.» | Ya estaban emparejados. | Para emparejarlos de nuevo, olvida antes este dispositivo en el otro. |
| «Los dos dispositivos tienen versiones distintas de la app.» | Una de las dos está desactualizada. | Actualiza ambas. |
| «El otro dispositivo ha tenido un problema.» | Un fallo en el otro. | Inténtalo de nuevo; si se repite, reinicia la app en los dos. |
| «Este dispositivo no está emparejado.» | El otro lo había olvidado. | Empareja de nuevo desde **Dispositivos**. |
| «Este dispositivo no está conectado a ninguna red.» | No hay wifi. | Conéctate a la wifi de casa. |

Debajo de estos avisos la app dice: **«No se ha perdido nada: se sincronizarán solos cuando se resuelva.»** Es verdad: nada de tu catálogo se pierde por un emparejamiento fallido. Más sobre esto en [Compartir en casa](../share-at-home/).

### Invitar y compartir una biblioteca

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «Este servidor no hace invitaciones.» | Tu servidor no las tiene activadas. | Puedes dar acceso con el QR. |
| «Solo quien administra la biblioteca puede invitar.» | No tienes ese rol. | Pídeselo a quien la administra. |
| «Esta biblioteca aún no puede invitar: necesita personas con roles y que la nube pueda vincular dispositivos.» | Falta algún ajuste en la biblioteca. | Mira que hay personas con roles y que está activada la opción de que la nube pueda dar acceso a dispositivos nuevos. |
| «Este enlace ya se usó.» / «Ha caducado: pide otro a quien administra.» / «Esta invitación se anuló.» | El enlace ya no vale. Vale una vez y 24 horas. | Pide uno nuevo. |
| «Esta invitación es para otra cuenta.» | Se invitó a otro correo. | Entra con la cuenta a la que se invitó. |
| «Tu cuenta ya es otra persona en esta biblioteca.» | Ya estás dentro con otro nombre. | No hace falta hacer nada. |
| «Este enlace es de otro servidor distinto al de tu cuenta. No se ha abierto.» | Estás conectado a un servidor y el enlace es de otro. | Cierra sesión y usa el servidor de la invitación. |
| «Eso no es un enlace de invitación.» / «Este enlace no vale.» | El texto pegado no es correcto o está incompleto. | Pega de nuevo el enlace completo. |
| «Esta biblioteca solo puede subirla quien la administra. Mientras, sigue compartiéndose por la red de casa.» | Subirla a la nube es cosa de quien administra. | Pídeselo, o sigue compartiéndola por la wifi. |
| «La nube dice que ya no tienes acceso a esta biblioteca.» | Quien la administra te quitó. | Habla con quien la administra. |
| «Esta biblioteca se empezó con una versión muy antigua de la app y no se puede subir tal cual.» | Es una biblioteca de pruebas muy antiguas. | Pulsa **Duplicar como nueva**: se crea otra con los mismos libros, y esa sí se puede subir. |

## Copias de seguridad

| Lo que ves | Qué significa | Qué hacer |
|---|---|---|
| «Este archivo no es una copia de seguridad de esta app.» | Has elegido un archivo que no es una copia. | Elige un archivo creado con la copia de seguridad de Kosmotheka. Un CSV se carga con **Importar CSV**. |
| «La copia está dañada y no se puede leer.» | El archivo está roto, o se cortó al descargarlo. | Prueba con otra copia, más antigua. Si la pasaste por correo o por la nube, descárgala otra vez. |
| «Esta copia se hizo con otra versión de la app.» | La copia es de una versión incompatible. | Actualiza la app y vuelve a intentarlo. |
| «Esta copia está protegida: escribe su contraseña.» | La copia tiene contraseña. | Escríbela. |
| «Contraseña incorrecta. No se ha restaurado nada.» | No coincide. | Prueba otra vez. Si la perdiste, **la copia no se puede abrir**: no hay forma de recuperarla. |
| «Escribe una contraseña para la copia.» | Marcaste proteger la copia y no pusiste contraseña. | Escríbela, o desmarca la protección (la copia quedaría legible para quien la encuentre). |
| «Esta copia es de una versión anterior y solo se restaura sobre una biblioteca vacía. Crea una biblioteca nueva, ábrela y restáurala ahí.» | Una copia antigua no se mezcla con una biblioteca que ya tiene cosas. | Haz lo que dice. |
| «Esta copia se hizo con una versión de desarrollo que usaba otro formato y no se puede abrir.» | Solo ocurre con copias de las primeras pruebas. | No hay forma de abrirla; haz una copia nueva. |

Más en [Copias de seguridad, exportar e importar](../backups/).

## Al añadir o editar un libro

Estos avisos salen en rojo debajo del campo, y el formulario no se cierra.

| Lo que ves | Qué hacer |
|---|---|
| «Este campo es obligatorio.» | Escribe al menos el título. Lo demás se puede completar después. |
| «Un ISBN tiene 10 o 13 dígitos.» | Cópialo de la contraportada o de la página de créditos. Los guiones y espacios no importan. |
| «Un ISBN solo puede tener dígitos (y una X final).» | Revisa que no se haya colado una letra. |
| «Este ISBN no es válido: revisa si hay una errata.» | Comprueba cada dígito, o escanéalo con la cámara. |
| «Esto no es un ISBN: los ISBN empiezan por 978 o 979. Busca el número junto al código de barras del libro.» | Busca el número pequeño que va junto al código de barras. |
| «Eso era una etiqueta de la tienda, no el ISBN.» | Escribe el número pequeño que empieza por 978 o 979, o vuelve a escanear apuntando a su código. |
| «Otro libro del catálogo ya tiene este ISBN.» | Si es el mismo libro, añade un ejemplar desde su ficha en lugar de cambiar este. |
| «Ya hay otra edición con este código de barras.» | Lo mismo: probablemente ya lo tienes. |
| «Este valor está fuera de rango.» / «No es una fecha válida.» | Revisa el año (entre 1 y 2100), las unidades o la fecha. |
| «*Servicio* ha rechazado la clave guardada. Revísala en Ajustes → Proveedores de datos.» | Pusiste tu propia clave de un catálogo y ya no vale. Pide una nueva, o bórrala para usar el servicio sin clave. |
| «Google Books ha rechazado la consulta (cuota agotada).» | Google limita las consultas sin clave. Pon una clave tuya, gratuita, en Ajustes. |

## Si el aviso no está aquí

- **«Algo ha fallado: …»** con un texto técnico: inténtalo otra vez. Si se repite, copia el texto tal cual y avisa a quien lleva el servidor o abre una incidencia en el proyecto.
- Cualquier cosa rara con tus datos: haz antes una [copia de seguridad](../backups/). Casi nada de lo que ves en esta página borra algo; si un aviso te pide hacerlo (por ejemplo dejar un dispositivo a otra persona), la copia te cubre.
