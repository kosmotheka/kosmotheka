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

## En inglés (en-US), por si Google lo pide

### Descripción corta (máximo 80 caracteres; esta tiene 78)

```
Catalog books, games, films and music at home. Account and cloud are optional.
```

### Descripción larga (máximo 4.000 caracteres; esta tiene 1.755)

```
Do we have that book? Where is Catan? What are we missing from that series?

Kosmotheka is the catalog of everything you have at home: books, comics, board games, films, TV series, video games, music and any other collection. You know what you have, where it is and whose each thing is.

MADE FOR SHARING
• Several libraries: the family's, the book club's, your own.
• Pair the phones and computers at home with a QR code and they catch up over your home network, with no servers.
• Permissions per person: who runs the library, who edits, and who only looks and asks for something to be added.

ADDING IS QUICK
• Scan the ISBN or search by title, and the details fill themselves in.
• Each kind has its own fields: players and length of a board game, director of a film, seasons of a series, platform of a video game.
• Do I have it? Scan it at the bookshop or the store before you buy it twice.
• Import your list from a spreadsheet (CSV).

EVERYONE'S OWN
• Each person keeps track of their own: to read, reading, read or dropped (or the same for each kind), their stars and their favorites.
• Filter by kind, by format, by owner or by what you still have to read.

THE CLOUD, IF YOU WANT IT
• An optional account (with email or with Google) to have your libraries on all your devices without sharing a wifi.
• Invite someone in another city with a link.
• For now, the cloud is by invitation. Without an account, everything works just the same.
• What you upload is stored on a server, and whoever runs it can read it. You choose which libraries to upload.

YOUR DATA IS YOURS
• Works without an account and offline.
• Everything is stored on your devices. No ads, no tracking.
• Backups whenever you want, and you can delete your account from the app.
```

## Formularios de Play Console

Todos, respondidos pregunta a pregunta (incluida «Seguridad de los datos», con ML Kit): [play-console-formularios.md](play-console-formularios.md) (17/10/2026). Texto de apoyo: [politica-privacidad-con-servidor.md](politica-privacidad-con-servidor.md), ya en la web.

## Pendiente para publicar

- El escáner usa **ML Kit** (Google, a través de Google Play Services) y se rige por sus términos ([ML Kit](https://developers.google.com/ml-kit/terms), [APIs de Google](https://developers.google.com/terms)). La política de privacidad y el formulario «Seguridad de los datos» tienen que nombrar ML Kit y sus términos de datos.
- URL de la política de privacidad: `https://kosmotheka.com/es/privacidad.html`, en la web del proyecto ([web.md](web.md)).
- URL para pedir que se borre la cuenta (la exige Google a toda app con cuentas): `https://kosmotheka.com/es/borrar-cuenta.html` (16/10/2026). Hay que subirla a la web antes de rellenar ese campo.
- **Hay que rellenar los formularios en Play Console** con [play-console-formularios.md](play-console-formularios.md) antes de publicar una versión con la cuenta. La clasificación de contenido, cuanto antes: sin ella, el control parental bloquea la app.
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
