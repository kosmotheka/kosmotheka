# Catálogo de errores, avisos y ayuda

> Cada problema que la app puede mostrar: qué ve el usuario, por qué pasa y qué debe hacer. Sirve al diseño (qué mensajes y ayudas hay que dibujar) y a la implementación (la columna **Código** es el identificador que envía la app). Patrón y niveles, en el [brief](brief.md#7-errores-avisos-y-ayuda-muy-importante).
>
> Niveles: **info** (neutral) · **aviso** (ámbar, no se perdió nada) · **error** (rojo, la acción no se hizo).

## Formularios (alta, edición, deseos)

Van debajo de cada campo, en rojo; el formulario no se cierra.

| Código | Mensaje | Qué hacer (texto de ayuda) |
|---|---|---|
| `required` | Este campo es obligatorio. | Escribe al menos el título; lo demás se puede completar después. |
| `isbn_length` | Un ISBN tiene 10 o 13 dígitos. | Cópialo de la contraportada o de la página de créditos. Los guiones y espacios no importan. |
| `isbn_character` | Un ISBN solo lleva números (y a veces una X al final). | Revisa que no se haya colado una letra; la «X» solo puede ir al final de un ISBN de 10. |
| `isbn_checksum` | Este ISBN no es válido. | Suele ser una errata: comprueba cada dígito, o escanéalo con la cámara. |
| `isbn_in_use` | Otro libro del catálogo ya tiene este ISBN. | Si es el mismo libro, añade un ejemplar desde su ficha en vez de cambiar este. **Botón:** «Abrir ese libro». |
| `out_of_range` | Este valor está fuera de rango. | Año entre 1 y 2100; unidades entre 1 y 999. |
| `invalid_date` | No es una fecha válida. | Elige la fecha en el calendario. |
| `invalid_cover_url` | No se puede usar esta portada. | Busca de nuevo por ISBN para tomar la portada de Google Books u Open Library. (Por seguridad solo se descargan portadas de esos servicios.) |

## Búsqueda de datos online

Son **avisos**, nunca errores: el alta sigue siendo posible a mano.

| Código | Mensaje | Qué hacer |
|---|---|---|
| (sin resultados) | No está en los catálogos online. | Rellénalo a mano; solo el título es obligatorio. Pasa con ediciones españolas poco comunes o muy antiguas. |
| `quota` (Google Books) | Google Books no ha respondido (límite de consultas). | Es un límite de Google para consultas sin clave. Para mejores resultados, crea una clave gratuita y ponla en Ajustes. **Botón:** «Cómo conseguir una clave». Mientras, se usan los datos de Open Library. |
| `network` | Sin conexión: no se pueden buscar datos online. | Revisa la conexión, o rellénalo a mano y búscalo más tarde desde «Editar». |
| `unexpected` | {Servicio} ha respondido algo inesperado. | Inténtalo más tarde; si se repite, puede que el servicio haya cambiado. |
| `cover_failed` | Libro guardado, pero sin portada. | La portada no se pudo descargar. Vuelve a buscar por ISBN desde «Editar» cuando haya conexión. |

## Escáner y cámara

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `camera_denied` | error | La app no tiene permiso para usar la cámara. | Ajustes del móvil → Aplicaciones → Library → Permisos → Cámara → Permitir. **Botón:** «Abrir ajustes». También puedes teclear el ISBN. |
| (cancelado) | — | *(nada: el usuario cerró la cámara)* | — |
| (código que no es un libro) | aviso | Eso no parece el código de un libro. | Busca el código de barras que empieza por 978 o 979, normalmente en la contraportada. Si no lo tiene (libros muy antiguos o de rol independientes), añádelo a mano. |

## ¿Lo tengo ya?

| Situación | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| Sin cobertura y no está en el catálogo | info | No está en vuestro catálogo. Sin conexión no se pueden ver sus datos. | Apúntalo en deseos con un título; los datos se completan cuando lo pases al catálogo con conexión. |
| Hace mucho que no se sincroniza | aviso | Este móvil no se sincroniza desde hace {n} días: puede que falten libros añadidos en otros dispositivos. | Abre la app en casa, en la misma wifi que el portátil. **Botón:** «Sincronizar ahora». |

## Copias de seguridad y CSV

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `not_a_backup` | error | Este fichero no es una copia de seguridad de la app. | Elige un `.zip` creado con «Guardar copia». Un CSV se carga con «Importar CSV». |
| `unreadable` | error | La copia está dañada y no se puede leer. | Prueba con otra copia más antigua. Si la has pasado por correo o por la nube, descárgala de nuevo. |
| `other_version` | error | Esta copia se hizo con otra versión de la app. | Actualiza la app a la última versión en este dispositivo e inténtalo de nuevo. |
| `not_empty` | error | Solo se puede restaurar en un catálogo vacío. | Restaurar reemplazaría lo que tienes. Si quieres **juntar** catálogos, empareja los dispositivos en «Dispositivos». Si de verdad quieres restaurar, hazlo en una instalación nueva. |
| (filas rechazadas en CSV) | aviso | Importados {n} libros; {m} filas no se pudieron importar. | Revisa las líneas indicadas (cada una dice qué campo falla), corrígelas en la hoja de cálculo e importa solo esas filas de nuevo. |
| `invalid_format` (CSV) | — | Formato desconocido en la línea {n}. | Usa: hardcover, paperback, pocket, ebook, audiobook u other (o déjalo vacío). |

## Dispositivos y sincronización

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `bad_code` | error | Eso no es un código de emparejamiento. | Copia el código completo (empieza por `LIBRARY1:`), o mejor, escanea el QR. |
| `same_device` | error | Ese código es de este mismo dispositivo. | El código se muestra en un dispositivo y se escanea o pega en **otro**. |
| `unreachable` (al emparejar) | error | No se encuentra el otro dispositivo. | Ver la guía **«El móvil no se empareja»** (abajo). **Botón:** «Ver qué revisar». |
| `unreachable` (al sincronizar) | aviso | No se pudo sincronizar con {nombre}: no está en la red. | Normal si estás fuera de casa o el otro está apagado; se sincronizará solo cuando coincidáis. Si estáis en la misma wifi, ver la guía. |
| `rejected` (al emparejar) | error | El código ya no es válido. | Los códigos valen **una vez** y **10 minutos**. En el otro dispositivo, pulsa «Mostrar código» otra vez. **Botón:** «Escanear otro código». |
| `rejected` (al sincronizar) | error | {nombre} ya no reconoce este dispositivo. | Alguien lo olvidó en el otro dispositivo, o se reinstaló la app allí. Olvídalo aquí y empareja de nuevo. **Botón:** «Emparejar de nuevo». |
| `wrong_certificate` | error | El dispositivo que ha respondido no es el esperado. | Por seguridad no se ha conectado. Si reinstalaste la app en el otro dispositivo, su identidad ha cambiado: olvídalo y empareja de nuevo. Si no, puede que otro equipo de la red esté usando esa dirección. |
| `other_version` | error | Los dos dispositivos tienen versiones distintas de la app. | Actualiza la app en ambos a la misma versión y sincroniza de nuevo. |
| `no_network` | error | Este dispositivo no está conectado a ninguna red. | Conéctalo a la wifi de casa para mostrar el código. |
| `peer_error` | error | El otro dispositivo ha tenido un problema. | Inténtalo de nuevo. Si se repite, reinicia la app en los dos dispositivos. |
| `unknown_device` | error | Este dispositivo ya no está emparejado. | Empareja de nuevo desde «Dispositivos». |

### Guía: «El móvil no se empareja» (o no sincroniza en casa)

Una pantalla de ayuda con comprobaciones, de la más probable a la menos. Cada una, con su explicación breve y, si es posible, un botón.

1. **¿Están en la misma wifi?** El móvil debe estar en la wifi de casa, no con datos móviles. Algunas casas tienen dos redes (por ejemplo, `Casa` y `Casa-5G`): normalmente se ven entre sí, pero no siempre.
2. **¿Es una red de invitados?** Las redes de invitados (y algunas de oficinas, hoteles o bibliotecas) aíslan los dispositivos entre sí a propósito. Usa la red principal.
3. **¿Está la app abierta en el otro dispositivo, con el código a la vista?** El código solo funciona mientras se muestra, durante 10 minutos, y una sola vez.
4. **¿Hay una VPN activa?** Una VPN en el portátil o en el móvil puede impedir que se vean en la red local. Desactívala mientras emparejas, o busca en ella la opción «permitir la red local» (en algunas se llama «LAN discovery»).
5. **¿El cortafuegos lo bloquea?** En Windows, la primera vez aparece un aviso del cortafuegos: permite el acceso en **redes privadas**. Si lo rechazaste: Configuración → Privacidad y seguridad → Seguridad de Windows → Firewall → Permitir una aplicación. En Linux, si usas un cortafuegos (`ufw`, por ejemplo), permite el puerto que muestra «Dispositivos».
6. **¿El móvil pide permiso de «red local» o «dispositivos cercanos»?** Algunas versiones de Android lo piden para conectar con equipos de casa: acéptalo.
7. **¿Es un router con «aislamiento de clientes»?** Algunos routers o sistemas wifi mesh lo traen activado. Se desactiva en la configuración del router.
8. **Último recurso:** reinicia la app en los dos dispositivos y genera un código nuevo.

*(Detalles técnicos plegados, para copiar si hay que pedir ayuda: direcciones que se probaron, puerto, versión de la app.)*

## Errores internos

| Código | Nivel | Mensaje | Qué hacer |
|---|---|---|---|
| `internal` | error | Algo ha fallado y no se ha podido completar. | Inténtalo de nuevo. Si se repite, copia los detalles (plegados) y avisa. **Nunca** mostrar el texto técnico como mensaje principal. |

## Pendiente de implementar

Hoy la app muestra estos mensajes de forma más escueta: la mayoría sin el «qué hacer», sin botones de acción y sin la guía de emparejamiento. También faltan situaciones que aún no detecta: el código de barras que no es de un libro, «hace mucho que no se sincroniza» y el aviso de sin conexión. Esta tabla es la referencia para implementarlos cuando llegue el nuevo diseño.
