# Formularios de Play Console, respondidos

> Preparado el 17/10/2026 para la app 0.2.1 (cuenta y nube opcionales, por invitación). Parte de [ficha-google-play.md](ficha-google-play.md) y de la política publicada ([web/es/privacidad.html](../../web/es/privacidad.html)), y de lo comprobado en el código ese día. Los nombres de los botones son los de Play Console en español y pueden variar un poco. **«Decides tú»** marca lo que solo sabe el maintainer, con una recomendación. Si la app cambia lo que sale del dispositivo, hay que repasar las secciones 2, 4 y 8.

Todo está en **Play Console › la app › Contenido de la aplicación** (en el menú, «Política y programas»), salvo las secciones 9 a 11.

## Comprobado en el código (17/10/2026)

- **Permisos de Android:** `INTERNET`, `CAMERA` y `VIBRATE` (estos dos, del escáner). Nada de ubicación, contactos, almacenamiento ni micrófono.
- **Informes de fallos o analítica propios: ninguno.** No hay Sentry, Crashlytics, Firebase ni nada parecido, ni en Rust ni en la interfaz ni en Gradle.
- **ID de publicidad:** ninguna dependencia lo usa (no está `play-services-ads-identifier` ni el permiso `AD_ID`).
- **ML Kit:** `play-services-mlkit-barcode-scanning` 18.1.0, a través de los servicios de Google Play, y sin el zoom automático. Según Google ([divulgación de datos de ML Kit](https://developers.google.com/ml-kit/android-data-disclosure)), ML Kit envía a Google datos técnicos en todas sus API: modelo y sistema del dispositivo, nombre y versión de la app, tiempos, errores e identificadores. **No envía las imágenes.** Hay que declararlo (sección 8).
- **El desbloqueo con huella** se queda en el dispositivo: Android no nos da nada.
- **Entrar con Google** pasa por el navegador y nuestro servidor (D77). La app no lleva el SDK de Google. Google nos da el correo, el nombre y un identificador de la cuenta.
- **El servidor guarda:**
  - de la cuenta: correo, nombre, idioma, la huella de la contraseña, el identificador de Google y el secreto del segundo factor;
  - de cada dispositivo: nombre, identificador, clave pública y cuándo se usó;
  - las bibliotecas que se suben;
  - el registro de actividad, con la IP y la versión de la app, durante un año;
  - la caché de metadatos, sin quién preguntó ni el texto buscado.

## 1. Política de privacidad

- **URL:** `https://kosmotheka.com/es/privacidad.html`.
- Play pide una sola. Pon la española: es la que dice «Responsable» con tu nombre. La inglesa está enlazada desde ella.
- **ML Kit en la política** (decidido el 17/10/2026): las dos políticas dicen ya que ML Kit manda a Google datos técnicos de uso, sin las imágenes. «Ni analítica» pasa a «ni analítica nuestra». Lo mismo en la página de la guía «Qué sale de tu dispositivo» y en el inicio de la web. **Sube `web/` antes de enviar el formulario de la sección 8.**

## 2. Acceso a la aplicación

> «¿Hay partes de la app que solo están disponibles con acceso restringido?»

- **Respuesta:** **«Algunas funciones o todas están restringidas».** El catálogo funciona sin cuenta, pero la cuenta y la nube piden iniciar sesión, y el servidor es por invitación. Si los revisores no pueden entrar, Google suele rechazar la versión o pedir acceso.
- **Hay que darles una cuenta de prueba** (decidido el 17/10/2026):
  1. Crea un buzón o alias tuyo, por ejemplo `review@kosmotheka.com`, que reenvíe a tu correo.
  2. Añádelo a `REGISTRATION_ALLOWED_EMAILS` en producción y regístrate con él desde la app. Confirma el código.
  3. Sin segundo factor: el revisor no tiene tu móvil.
  4. Sube a la nube una biblioteca de prueba con cuatro o cinco obras, para que vean algo.
  5. La contraseña, solo en Play Console y en tu gestor. Nunca en el repositorio ni en el chat.
- **Instrucciones para el revisor** (en inglés, que es lo que leen):

  ```
  The whole catalog works without an account: open the app, add a book by ISBN or title.
  The optional account and cloud (Settings › Account) are invitation-only on our server.
  Sign in with "Sign in or create account" › email and password, using the credentials above.
  The account has one cloud library ("Review library") to see sync and sharing.
  ```

- **Nombre de usuario:** el correo de prueba. **Contraseña:** la tuya. **Otra información:** nada.
- Si cambias la contraseña o borras la cuenta, actualízalo aquí.

## 3. Anuncios

> «¿Tu app contiene anuncios?»

**No.** No hay ningún SDK de anuncios. La ficha y la política ya lo dicen.

## 4. Clasificación de contenido (cuestionario IARC)

1. **Correo de contacto:** `info@kosmotheka.com`.
2. **Categoría:** **«Todas las demás apps»** (utilidad, productividad, comunicación u otros). No es un juego ni una red social: el catálogo solo lo ven las personas invitadas a cada biblioteca.
3. **Preguntas de contenido:**

   | Pregunta | Respuesta | Por qué |
   |---|---|---|
   | ¿Violencia, sangre o miedo? | No | No hay contenido así. Las portadas vienen de catálogos públicos y se buscan a petición |
   | ¿Sexualidad o desnudos? | No | |
   | ¿Lenguaje malsonante? | No | |
   | ¿Drogas, alcohol o tabaco? | No | |
   | ¿Humor grosero? | No | |
   | ¿Apuestas, o juegos de azar simulados? | No | |
   | ¿Contenido sobre odio o discriminación? | No | |

4. **Elementos interactivos:**

   | Pregunta | Respuesta | Por qué |
   |---|---|---|
   | ¿Los usuarios pueden interactuar o intercambiar contenido? | **Sí** | Con la nube, una biblioteca se comparte con personas invitadas, y ven lo que apunta cada una: títulos, notas, nombres. Es lo honesto, y solo añade la nota «Los usuarios interactúan»; no sube la edad |
   | ¿Hay contenido generado por usuarios visible para otros? | Sí, **solo entre personas invitadas** | No hay nada público ni perfiles que pueda buscar un desconocido. Si pregunta por moderación: cada biblioteca la administra quien la creó, que puede quitar a cualquiera, y hay un correo de contacto |
   | ¿Se comparte la ubicación del usuario? | No | La app no pide ubicación |
   | ¿Se pueden comprar bienes digitales? | No | Es gratis, sin compras dentro de la app |
   | ¿Acceso libre a internet, como un navegador o un buscador? | No | Solo habla con los catálogos que se configuran y con el servidor |

5. **Resultado esperado:** PEGI 3, con la nota «Los usuarios interactúan». Si sale otra cosa, dímelo antes de aceptarla.

**Comprobado el 17/10/2026:** en la prueba interna, aunque el cuestionario esté enviado, la clasificación no se aplica, porque esa pista no pasa revisión. Para Google Play, una app sin clasificar es **solo para mayores de 18**. Una cuenta de menor no puede instalarla ni aunque Family Link permita todas las apps: da error de edad. Los testers menores tienen que esperar a la **prueba cerrada**, que sí pasa revisión y publica la clasificación (PEGI 3).

## 5. Público objetivo y contenido

**Dos cosas distintas que no se contradicen:**

| | Qué dice | Quién lo ve | Qué hace |
|---|---|---|---|
| **Clasificación (PEGI 3, sección 4)** | Si el **contenido** es apto para una edad: no hay nada que un niño no deba ver | Se enseña en la ficha | Es lo que mira el control parental: un filtro «PEGI 12» deja instalar una app PEGI 3. Sin clasificar, la bloquea |
| **Público objetivo (aquí)** | Para quién está **pensada** la app | Solo Google; no sale en la ficha | Si incluye menores de 13, la app entra en la política de familias. **No impide instalarla a nadie** |

La cuenta desde los 14 es una tercera cosa: la edad para dar el consentimiento propio en España (política, «Menores»). Sin cuenta, la app la usa cualquiera en casa.

1. **Grupos de edad:** **13-15, 16-17 y 18 o más.** Sin los de menos de 13, que obligarían a la política de familias (SDK certificados y revisión aparte). Y la app no está pensada para ellos, aunque puedan usarla en casa.
2. **«¿Podría tu app atraer a niños sin querer?»** **No.** No tiene personajes, ni animaciones, ni lenguaje infantil.
3. **Presencia en la tienda:** nada más que marcar.

**A probar (17/10/2026):** tu hijo, de 14 años, no pudo instalarla con el filtro PEGI 12 porque aún estaba sin clasificar. Después de enviar la sección 4:
1. Comprueba en la ficha que sale PEGI 3.
2. Vuelve a intentarlo en su móvil.
3. Si sigue sin dejar, mira dos cosas:
   - que su cuenta esté en la lista de testers;
   - si Family Link pide aprobar la app a mano.

   Apúntame lo que pase.

## 6. Noticias, salud, finanzas, gobierno y otras declaraciones

| Formulario | Respuesta |
|---|---|
| Apps de noticias | **No** es una app de noticias |
| Apps de salud | **«Mi app no tiene funciones de salud»** |
| Funciones financieras | **«Mi app no ofrece ninguna función financiera»** |
| Apps gubernamentales | **No** está hecha por ni para un gobierno |
| Rastreo de contactos o estado de COVID-19 (si sale) | **No** |
| Permisos sensibles (si sale) | Nada que declarar: la cámara no es un permiso que Play pida justificar |

## 7. ID de publicidad

> «¿Tu app usa el ID de publicidad?»

**No.** Ninguna dependencia lo usa y el permiso `AD_ID` no está en el manifiesto. Comprueba que sigue así en cada versión nueva: en el `.aab`, Play avisa si aparece.

## 8. Seguridad de los datos

### 8.1 Preguntas generales

| Pregunta | Respuesta | Por qué |
|---|---|---|
| ¿Tu app recoge o comparte alguno de los tipos de datos obligatorios? | **Sí** | Con la cuenta, y siempre por ML Kit al escanear |
| ¿Todos los datos que recoge se cifran en tránsito? | **Sí** | El servidor solo va por HTTPS, y ML Kit también. La sincronización en casa va entre tus dispositivos, cifrada, y no nos llega |
| ¿Qué formas de crear una cuenta admite? | **«Nombre de usuario y contraseña»** (el correo hace de nombre) y **«OAuth»** (Google) | |
| Enlace para pedir que se borre la cuenta | `https://kosmotheka.com/es/borrar-cuenta.html` | **Tiene que estar subida antes de enviar** |
| ¿Se puede pedir que se borren algunos datos sin borrar la cuenta? | **Sí** | Por correo a `privacy@kosmotheka.com` («Tus derechos» de la política) |
| ¿Validación de seguridad independiente (MASA)? | **No** | No se ha hecho |
| ¿La app cumple la política de familias? | No aplica | Sin menores de 13 (sección 5) |

### 8.2 Tipos de datos

**Marca solo estos.** Los demás («Ubicación», «Información financiera», «Salud», «Mensajes», «Fotos y vídeos», «Audio», «Archivos», «Calendario», «Contactos», «Navegación web», «Información sobre apps instaladas», «Registros de fallos») van **sin marcar**.

| Categoría › tipo | ¿Recogido? | ¿Compartido? | ¿Efímero? | ¿Obligatorio? | Finalidades | Qué es |
|---|---|---|---|---|---|---|
| Información personal › **Nombre** | Sí | No | No | **Opcional** | Funcionalidad de la app · Gestión de la cuenta | El nombre de la cuenta, que ven las personas de una biblioteca compartida |
| Información personal › **Dirección de correo** | Sí | No | No | Opcional | Gestión de la cuenta · Funcionalidad de la app | El de la cuenta (códigos, recuperar la contraseña). La dirección de alguien a quien invitas no se guarda: solo su huella |
| Información personal › **ID de usuario** | Sí | **Sí** (desde la 0.4.0) | No | Opcional | Recogido: gestión de la cuenta · Prevención de fraudes, seguridad y cumplimiento. Compartido: funcionalidad de la app | El identificador de la cuenta y, si entras con Google, el de Google. Se comparte con GameUPC una huella de la cuenta (ver abajo) |
| Actividad en la app › **Historial de búsqueda en la app** | Sí | No | **Sí** | Opcional | Funcionalidad de la app | Con cuenta, lo que buscas por título pasa por el servidor, que no lo guarda (Ajustes › Proveedores lo apaga) |
| Actividad en la app › **Otro contenido generado por el usuario** | Sí | No | No | Opcional | Funcionalidad de la app | Las bibliotecas que subes a la nube: obras, ejemplares, personas, notas |
| Actividad en la app › **Otras acciones** | Sí | No | No | Opcional | Prevención de fraudes, seguridad y cumplimiento | El registro de actividad de la cuenta: entradas, cambios de contraseña, quién quitó a quién. Un año |
| Información y rendimiento de la app › **Diagnóstico** | Sí | No | No | Opcional | Analíticas | ML Kit al escanear: modelo, versiones, tiempos y errores (los recibe Google, no nosotros) |
| Dispositivo u otros identificadores › **Dispositivo u otros identificadores** | Sí | No | No | Opcional | Gestión de la cuenta · Prevención de fraudes, seguridad y cumplimiento · Analíticas | El identificador y la clave pública de cada dispositivo con cuenta, y el de la sesión de escaneo de ML Kit |

Por qué es así:
- **«Compartido: sí» solo en «ID de usuario», desde la 0.4.0 (decidido el 24/10/2026).** Al guardar un juego de mesa elegido entre las propuestas de GameUPC (o buscado tras escanear un código que no conocía), el servidor le envía el código, el juego y una huella de la cuenta: no dice quién eres, pero es siempre la misma para cada cuenta, y Play solo deja fuera los datos anónimos del todo. La excepción de «acción iniciada por el usuario» se podría defender, pero es discutible; se declara. Coincide con la política de privacidad.
- **«Compartido: no» en lo demás.** DigitalOcean (alojamiento) y Brevo (correo) son encargados que tratan los datos por cuenta nuestra, y eso no cuenta como compartir (lo dice la ayuda de Play). Lo que se busca en Open Library, Google Books, TMDB, BoardGameGeek y los demás catálogos se envía porque tú lo pides al buscar, y eso tampoco cuenta.
- **«Opcional» en todo.** La app funciona entera sin cuenta y sin escanear: se puede escribir el ISBN o el título.
- **La IP no es un tipo de dato del formulario.** Solo habría que declararla como «Ubicación aproximada» si se usara para saber dónde estás, y no se usa para eso. La guarda el registro de actividad, por seguridad, y ya lo dice la política.
- **Sin cuenta, la app no nos envía nada.** Las búsquedas van directas a los catálogos, sin nada que diga quién eres.

ML Kit con «Analíticas» es lo que Google indica para su biblioteca. La ficha sigue diciendo «Sin publicidad ni seguimiento»: esos datos los recibe Google, no nosotros, y la política ya lo cuenta (sección 1).

## 9. Ficha de Play Store (Hacer crecer › Presencia en Google Play › Ficha principal)

| Campo | Valor |
|---|---|
| Nombre de la app | **Kosmotheka** |
| Descripción corta | La de [ficha-google-play.md](ficha-google-play.md) (78 caracteres) |
| Descripción larga | La de [ficha-google-play.md](ficha-google-play.md) (1.776 caracteres) |
| Icono | 512 × 512, de `brand/` |
| Gráfico destacado | 1024 × 500, de `brand/` |
| Capturas de móvil | Al menos 2, mejor 4 a 8: el catálogo, una ficha, añadir con el escáner y la nube |
| Capturas de tablet | Opcionales; sin ellas no sale destacada en tablets |
| Idiomas | Español (España) como principal. El inglés (en-US), con las descripciones de [ficha-google-play.md](ficha-google-play.md), está listo por si Google lo pide o quieres añadirlo |

## 10. Configuración de la tienda (Presencia en Google Play › Configuración de la tienda)

| Campo | Valor | Por qué |
|---|---|---|
| App o juego | **App** | |
| Categoría | **Libros y obras de consulta** | Empezó por los libros y es lo que más se busca (decidido el 17/10/2026) |
| Etiquetas (hasta 5, de la lista de Google) | Elige de la lista las que más se acerquen a: libros, lectura, colecciones, organización, juegos de mesa | Solo vale lo que ofrece la lista; las elegimos juntos al rellenarlo |
| Correo de contacto | `info@kosmotheka.com` | Obligatorio y público |
| Teléfono | Vacío | Opcional y público |
| Sitio web | `https://kosmotheka.com` | |
| Política de privacidad | La misma URL de la sección 1 (a veces se pide también aquí) | |

## 11. Países, precio y pistas

- **Precio:** **Gratis.** Una app gratuita no puede pasar a ser de pago después; para nosotros no importa.
- **Países:**
  - **Todos los disponibles** (decidido el 17/10/2026). La política vale en cualquier sitio (RGPD, el más exigente) y la app no depende del país.
  - Las pistas de prueba (interna y cerrada) tienen su propia lista de países: revísala también.
- **Prueba cerrada:**
  - si la cuenta de desarrollador es personal y nueva, 12 personas durante 14 días antes de producción ([ficha-google-play.md](ficha-google-play.md));
  - cada tester tiene que estar en la lista con su cuenta de Google;
  - las cuentas supervisadas (Family Link) necesitan además que la app esté clasificada (sección 4).

## Orden recomendado para rellenarlo juntos

1. Sube `web/`: la política con ML Kit, el inicio y la página de borrar la cuenta.
2. Clasificación de contenido (sección 4): desbloquea el control parental ya.
3. Público objetivo, anuncios, ID de publicidad y las declaraciones de «no» (secciones 3, 5, 6 y 7).
4. Crea la cuenta de revisión y rellena «Acceso a la aplicación» (sección 2).
5. Seguridad de los datos (sección 8).
6. La ficha, la configuración de la tienda y los países (secciones 9 a 11).
