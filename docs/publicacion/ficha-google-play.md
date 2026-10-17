# Ficha de Google Play

> Redactada el 29/09/2026 para la primera publicación (prueba cerrada) y puesta al día el 17/10/2026 con la cuenta y la nube (por invitación). Solo cuenta lo que ya tiene interfaz.

## Descripción corta (máximo 80 caracteres; esta tiene 78)

```
Cataloga en casa libros, juegos, películas y música. Cuenta y nube opcionales.
```

## Descripción larga (máximo 4.000 caracteres; esta tiene 1776)

```
¿Ese libro lo tenemos? ¿Dónde está el Catán? ¿Qué nos falta de esa serie?

Kosmotheka es el catálogo de todo lo que hay en casa: libros, cómics, juegos de mesa, películas, series, videojuegos, música y cualquier otra colección. Sabes qué tenéis, dónde está y de quién es cada cosa.

HECHO PARA COMPARTIR
• Varias bibliotecas: la de la familia, la del club de lectura, la tuya.
• Empareja los móviles y ordenadores de casa con un código QR y se ponen al día por la red local, sin servidores.
• Permisos por persona: quien administra, quien edita y quien solo consulta y pide que se añada algo.

AÑADIR ES RÁPIDO
• Escanea el ISBN o busca por título, y los datos se rellenan solos.
• Cada tipo tiene su ficha: jugadores y duración de un juego de mesa, dirección de una película, temporadas de una serie, plataforma de un videojuego.
• ¿Lo tengo? Escanea en la librería o en la tienda antes de comprarlo repetido.
• Importa tu lista desde una hoja de cálculo (CSV).

LO DE CADA UNO
• Cada persona apunta lo suyo: por leer, leyendo, leído o abandonado (o lo equivalente en cada tipo), sus estrellas y sus favoritos.
• Filtra por tipo, por formato, por dueño o por lo que te queda por leer.

LA NUBE, SI LA QUIERES
• Una cuenta opcional (con correo o con Google) para tener tus bibliotecas en todos tus dispositivos sin compartir wifi.
• Invita a alguien de otra ciudad con un enlace.
• Por ahora, la nube es por invitación. Sin cuenta, todo sigue funcionando igual.
• Lo que subes se guarda en un servidor, y quien lo mantiene puede leerlo. Tú decides qué bibliotecas subir.

TUS DATOS SON TUYOS
• Funciona sin cuenta y sin conexión.
• Todo se guarda en tus dispositivos. Sin publicidad ni seguimiento.
• Copias de seguridad cuando quieras, y puedes borrar tu cuenta desde la app.
```

## Seguridad de los datos (formulario de Play Console)

Con la cuenta y la nube (17/10/2026) la app **recoge datos, solo si se crea una cuenta**, y **no los comparte** con terceros para sus propios fines (el correo lo envía Brevo y el servidor está alojado en DigitalOcean: son encargados del tratamiento). A declarar:

- **Correo electrónico y nombre** (gestión de la cuenta).
- **Contenido generado por el usuario**: el catálogo de las bibliotecas que se suben (funcionalidad de la app).
- Cifrado en tránsito: **sí**. Se puede pedir el borrado: **sí**, desde la app y por correo (`https://kosmotheka.com/es/borrar-cuenta.html`). La recogida es **opcional**.
- Sin cuenta, la app no recoge nada: las búsquedas (un ISBN, un código de barras o un título) no son datos personales, y la sincronización en casa va entre los dispositivos del usuario, cifrada.

Texto de apoyo: [politica-privacidad-con-servidor.md](politica-privacidad-con-servidor.md), ya en la web.

## Pendiente para publicar

- El escáner usa **ML Kit** (Google, a través de Google Play Services) y se rige por sus términos ([ML Kit](https://developers.google.com/ml-kit/terms), [APIs de Google](https://developers.google.com/terms)). La política de privacidad y el formulario «Seguridad de los datos» tienen que nombrar ML Kit y sus términos de datos.
- URL de la política de privacidad: `https://kosmotheka.com/es/privacidad.html`, en la web del proyecto ([web.md](web.md)).
- URL para pedir que se borre la cuenta (la exige Google a toda app con cuentas): `https://kosmotheka.com/es/borrar-cuenta.html` (16/10/2026). Hay que subirla a la web antes de rellenar ese campo.
- **La ficha y el formulario «Seguridad de los datos» se rehicieron el 17/10/2026** para la cuenta y la nube (arriba); hay que **rellenar el formulario en Play Console con lo de arriba** antes de publicar una versión con la cuenta.
- Icono de 512 × 512 y gráfico destacado de 1024 × 500 (en `brand/`), al menos dos capturas de móvil.
- Si la cuenta de desarrollador es personal y nueva: prueba cerrada con al menos 12 personas durante 14 días antes de producción.
- Antes de la primera subida: confirmar el identificador de la app (no se puede cambiar después), crear la clave de subida fuera del repositorio y compilar el `.aab`.

## La clave de subida (acordado el 02/10/2026; se hace en una sesión juntos)

Con «Play App Signing», Google guarda la clave que firma la app instalada; el maintainer solo tiene la **clave de subida**, que demuestra que el `.aab` es suyo (si se pierde, Google permite cambiarla). La contraseña nunca debe estar al alcance del agente:

- La clave (`.jks`) y el fichero con su contraseña viven **fuera del repositorio**, en una carpeta del maintainer (por ejemplo `~/.kosmotheka-release/`), y además se ignoran en `.gitignore` por si acaso.
- El maintainer añade una regla de permisos de Claude Code que **prohíbe** leer esa carpeta.
- La clave la crea el maintainer con `keytool`, escribiendo él la contraseña; la compilación de *release* también la lanza él, en su terminal. El agente prepara la configuración de Gradle, la orden exacta y la guía.
- La contraseña, en el gestor de contraseñas; una copia del `.jks`, fuera del ordenador.
- Identificador actual: `app.kosmotheka` (confirmarlo antes de la primera subida).
- Pasos concretos: [firma-android.md](firma-android.md).
