# Revisión del alcance y modelo de bibliotecas

> **Estado:** acordado en conversación el 25/09/2026; todavía **no son decisiones**. Falta revisar los catálogos (sección 13). Con eso claro se rehará el plan y lo acordado pasará a [DECISIONS.md](../DECISIONS.md). Mientras tanto, el [plan de la Fase 3](fase-3.md) queda en pausa.

## 1. Por qué esta revisión

Al plantear una versión web (D53) salió a la luz que el proyecto no estaba aterrizado. Se dio por hecho que cada persona tendría varias bibliotecas (una personal de rol, una familiar, la de videojuegos del hijo) y no se reflejó en el diseño: la app actual trata el dispositivo entero como una sola biblioteca. Además, la Fase 3 había ido creciendo hacia un servicio público (registro abierto, superadministrador, KPI) antes de resolver lo que la familia necesita. Se empezó a construir demasiado pronto.

## 2. Qué producto es

### Tres círculos y un objetivo personal

| Círculo | Qué es | Cuándo |
|---|---|---|
| **1. Tu gente** | Tu familia y tus amigos, en sus dispositivos | Lo primero; manda sobre todo lo demás |
| **2. Código abierto autoalojable** | Cualquiera se instala la app, y quien quiera monta su propio servidor | Casi gratis si el círculo 1 está bien hecho |
| **3. Servicio en la nube** | Tu servidor, abierto a amigos lejanos y, quizá, a ludotecas, clubes o colegios, incluso de pago | Se prepara la arquitectura ahora; se construye después |

**Objetivo personal:** aprender Rust y Tauri creando apps nativas. Pensar a lo grande sirve para ver antes los problemas de escalabilidad: si la arquitectura está preparada, crecer y abrirlo a cualquiera sale casi gratis.

### Dos formas de usar la app, las dos de primera

- **Sin cuenta:** gratis, sin registro, sin nube. Sincroniza por la red local y guarda copias donde quiera (Google Drive). Es la opción de la privacidad y **tiene que seguir existiendo siempre**.
- **Con cuenta:** un botón de registro con el email, y todo se guarda y sincroniza en la nube, también a distancia y sin cobertura (se envía al recuperarla).

### Escenarios que tiene que cubrir

1. **Casa:** Linux (portátil), Windows (el ordenador del hijo) y Android (todos). En una tienda, una foto del libro que quieres que te regalen. Varias bibliotecas, y la familia solo accede a lo que tú eliges.
2. **Amigos en el bar:** emparejar móviles para compartir una biblioteca, sin registro.
3. **Amigos lejos:** una biblioteca común con amigos de otras ciudades, prestándose libros por correo.
4. **Otra familia, aislada:** un amigo comparte con su familia sus bibliotecas; nadie ve lo de nadie.
5. **Instalación:** APK durante las pruebas; después, tienda de Android. iOS queda fuera. Una landing explica el proyecto y enlaza a la tienda.
6. **Desconocidos:** una familia de EE. UU. la usa sin registro y sin que tú intervengas.
7. **Contribuciones:** alguien monta el proyecto en su casa, hace cambios y manda un PR.
8. **Servicio:** una ludoteca, un club o un colegio la quiere con usuarios registrados sin montar su servidor.

## 3. La biblioteca

- **Una biblioteca es lo que se comparte.** Es la unidad de todo: qué se sincroniza, con quién y con qué permisos, con cuenta y sin ella. Tus juegos de rol, la familiar y los videojuegos de tu hijo son tres bibliotecas.
- **Vive completa en cada sitio que la tiene:** tus dispositivos, los de quienes la comparten contigo y, si quieres, la nube. **Compartir es añadir un sitio más** donde vive: emparejar un dispositivo, invitar a un amigo o subirla a la nube son, en el fondo, lo mismo.
- **Tiene un propietario**, que hoy siempre es una persona. Es la puerta para las organizaciones (sección 8) sin tener que construirlas.
- Un dispositivo puede tener varias bibliotecas, y cada una decide con quién se comparte: el móvil de un amigo recibe solo la biblioteca que le compartes.

## 4. Personas, dispositivos y roles

### Personas y dispositivos

- **La persona de la biblioteca es el miembro**, que ya existe en la app como dueño de los ejemplares.
- Al compartir con un dispositivo se dice **de quién es** («el móvil de Hugo» es de Hugo). Una persona puede tener varios dispositivos.
- **Los permisos, las lecturas y los ejemplares son de la persona**, no del dispositivo. «Leído» lo marca Hugo, sea desde el móvil o desde el PC.
- En la nube, el miembro se enlaza con una cuenta.

### Roles

| Rol | Puede |
|---|---|
| **Administrador** | Todo: editar, borrar, dar permisos, compartir y gestionar las peticiones |
| **Editor** | Crear y editar el catálogo |
| **Lector** | Ver, marcar sus lecturas y **pedir** que se añada o se borre algo |

- **Nadie da más de lo que tiene.** Un lector no puede hacer editor a nadie.
- **Compartir es un permiso aparte**, que no se tiene por defecto. Tu hijo puede **enseñar** la biblioteca familiar a sus amigos en su pantalla, pero no **compartirla**: nadie entra en la familia sin tu permiso.
- **Puede haber varios administradores** (tu móvil y tu portátil, o tú y tu pareja), para no depender de un solo dispositivo.
- Son **los mismos roles con cuenta y sin ella**.

### Peticiones

- Lo que un rol no puede hacer se **pide**: tu hijo ve un libro en la biblioteca del instituto y pide añadirlo; quiere quitar uno y pide borrarlo.
- Una sección de peticiones que gestionan los administradores: aceptar aplica el cambio; rechazar lo descarta.
- **Si a alguien le quitan un permiso mientras está sin conexión**, lo que haga sin ese permiso se convierte automáticamente en petición cuando llega.

## 5. Cómo se garantiza sin servidor

- Cada dispositivo ya se identifica por su certificado al emparejar (D34).
- **Firmas:** cada dispositivo tiene un par de claves (Ed25519) y firma sus operaciones; los demás las comprueban con su clave pública. Así se sabe quién hizo cada cambio, aunque llegue a través de otro dispositivo. Es lo que hacen SSH o Signal; nada de blockchain.
- **Cada dispositivo comprueba lo que recibe** contra los roles de la biblioteca, que también se sincronizan, y convierte en petición lo que no esté permitido.
- **La parte delicada:** todos los dispositivos tienen que llegar a la misma conclusión aunque les lleguen las cosas en distinto orden. Si a uno le llega primero un cambio y después la retirada del permiso con fecha anterior, tiene que reconsiderar lo que ya aplicó. Es el corazón del nuevo motor de sincronización y se diseñará con casos de prueba compartidos en `spec/`, como la fusión.
- Entre conocidos se confía en el reloj de cada cambio (HLC); alguien podría trucarlo, y se acepta.

## 6. Cómo viaja una biblioteca

- **Red local (wifi):** como hoy, pero por biblioteca. Los dispositivos solo se ponen al día cuando coinciden en la misma red.
- **En la calle o en un bar:** uno comparte datos (punto de acceso del móvil) y el otro se conecta; así están en la misma red y el emparejamiento funciona sin programar nada. **Bluetooth directo se descarta**: Tauri no tiene plugin oficial, y lo que usa Android para pasar fotos (Quick Share) no llega a Linux ni a Windows.
- **A distancia:** solo con la nube.

## 7. La nube

- **La nube es un dispositivo más, siempre encendido y en internet**, con los mismos roles. Resuelve a la vez la copia en la nube, la distancia y el paso de anónimo a registrado sin perder nada: la app sube lo que ya tiene.
- **Registro con el email**, verificado con un código que se teclea en la app (D49). La identidad es la cuenta, no el dispositivo, porque al cambiar de móvil o reinstalar el dispositivo cambia.
- **Un servidor que entiende los datos**, no un relé cifrado: ludotecas y colegios necesitan que el servidor compruebe permisos y, quizá, una web. La privacidad total la da el modo sin cuenta.
- El servidor Symfony, el panel del superadministrador (D51, D52) y la landing pertenecen a este mundo.

## 8. El nivel de organización

- Un nivel por encima de las bibliotecas, como las organizaciones de GitHub: la propiedad sobrevive a las personas, hay un directorio de gente al que se dan accesos, y administración central (facturación, inicio de sesión con la cuenta del colegio).
- **No aparece en el uso normal.** Compartir con la familia tiene que ser sencillo y sin nada que suene a corporativo; en el móvil sería ruido.
- **Se deja la puerta abierta:** la biblioteca pertenece a un propietario que hoy es una persona y mañana puede ser una organización.

## 9. Copias y recuperación

- **Exportar y guardar una copia tiene que ser muy fácil y estar a la vista** (por ejemplo, en Google Drive), con copias completas y por biblioteca (D46). Sincronizar no es hacer copia: un borrado por error también se sincroniza.
- **La copia no lleva claves ni necesita contraseña:**

| Si pierdes el móvil y… | Recuperas el control |
|---|---|
| Tienes cuenta en la nube | Con tu email |
| Hay otro administrador (tu otro dispositivo, tu pareja) | Él revoca el perdido y da permisos al nuevo |
| La biblioteca es personal, solo tuya | Restauras la copia y pasa a ser tuya con una clave nueva |
| **Es compartida y tú eras el único administrador, con un solo dispositivo** | **El único hueco** |

- Salidas para el hueco: un código de recuperación guardado al crear la biblioteca (como los de la verificación en dos pasos); que la app sugiera un segundo administrador al compartir; o, en último caso, duplicarla como nueva y volver a emparejar.
- La biometría no ayuda: sus claves no salen del chip del móvil. Google Block Store sí las traslada a un móvil nuevo, pero solo en Android con servicios de Google.

## 10. Lo que no sale gratis (sin que pare el proyecto)

Aunque la arquitectura esté preparada, el servicio en la nube tiene costes propios:
- **Correo fiable:** si cae en spam, nadie se registra. Proveedor por decidir (quizá Cloudflare).
- **Portadas a escala:** lo que más ocupa.
- **Protección de datos:** con colegios hay datos de menores y un contrato de encargado del tratamiento con cada centro.
- **Inicio de sesión corporativo** (Google, Microsoft) para colegios y organizaciones.
- **Una web para el personal** de una ludoteca o un colegio.

## 11. Distribución, código abierto, web e iOS

- **Android:** tienda de Google (pago único como desarrollador) y **F-Droid**, el canal del software libre. **Por comprobar:** las nuevas exigencias de Google de verificar a los desarrolladores, también para las APK instaladas a mano.
- **Código abierto y tienda son compatibles.** Signal, Bitwarden, Thunderbird, Immich o Nextcloud publican su código y están en la tienda. Solo tú publicas con tu nombre e identificador; otros hacen un fork con otro identificador o generan su APK. Lo que aporta es confianza, contribuciones y autoalojamiento.
- **Libre y con servicio de pago** es un modelo conocido (Nextcloud, Bitwarden, Plausible, Ghost). La AGPL (D11) lo protege: quien monte tu servidor como servicio tiene que publicar sus cambios.
- **Web:** solo aparece en el escenario 8 (el personal en un ordenador de mostrador). En los demás no hace falta. D53 sigue abierta, sin urgencia.
- **iOS:** fuera por ahora. Tauri 2 lo permite, pero exige un Mac con Xcode (o la CI con macOS) y una cuenta de desarrollador de Apple (99 $ al año).
- **Realismo sobre el escenario 8:** las bibliotecas municipales ya tienen sistemas consolidados (AbsysNet, Koha); ludotecas, clubes, asociaciones y colegios son un público más realista.

## 12. Impacto en lo construido

### La app (fases 1 y 2)

| Qué hay | Qué cambia |
|---|---|
| Tablas del catálogo sin biblioteca | `library_id` en todas y una tabla de bibliotecas (D46) |
| `members`: solo un nombre | Pasa a ser la persona: dispositivos asociados, rol y, en la nube, cuenta enlazada |
| `operations` sin biblioteca ni firma | Cada operación pertenece a una biblioteca y va firmada por su dispositivo |
| Vector de versiones por dispositivo, de toda la base | Por biblioteca |
| Emparejamiento que lo comparte todo (`known_servers`, `known_clients`) | Se comparte biblioteca a biblioteca, con persona y rol |
| `sync::merge` aplica todo lo recibido | Comprueba roles y firmas, y convierte en petición lo no permitido |
| Copias y CSV de toda la base | Completas y por biblioteca |
| Sin lecturas, sin peticiones | Nuevas |

Lo demás sirve tal cual o casi: validación, ISBN, proveedores de metadatos, portadas, búsqueda, escáner, HLC, fusión campo a campo y el transporte HTTPS con certificado fijado.

### `crates/domain` y `spec/`

El formato de operación gana la biblioteca y la firma; hacen falta casos de conformidad nuevos para los roles, las peticiones y el orden de llegada.

### El servidor (hito 3.0)

El esqueleto, Docker, la calidad y la validación en PHP siguen valiendo; la validación se adapta al nuevo formato.

### Decisiones que hay que revisar

- **D05** (servidor con autoridad): los permisos también se garantizan sin servidor; la autoridad del servidor queda para las bibliotecas en la nube.
- **D39** (biblioteca en la nube: solo se sincroniza con el servidor): su motivo era que sin servidor no había permisos. Ahora los dispositivos también los comprueban; hay que decidir si una biblioteca en la nube puede seguir sincronizándose por wifi.
- **D40** (la app aplica los permisos y el servidor los recomprueba): encaja, y se amplía a todos los dispositivos.
- **D42–D45, D47:** se reescriben con el modelo de personas, roles y peticiones.
- **D46:** se reescribe con este documento.
- **D49, D50:** el registro con email; revisar si hace falta contraseña o basta el código por email.
- **D41, D48, D51, D52:** siguen valiendo, para el mundo de la nube.
- **D13, D31–D34, D36:** se amplían (sincronización por biblioteca, firmas).

## 13. Queda abierto

- **Catálogos:** rol, novelas, películas, series, juegos de mesa y videojuegos; cuáles y en qué orden; los libros de rol sin ISBN o en PDF. **Es lo siguiente que se revisa.**
- ¿Una **papelera** para deshacer borrados? Hoy el borrado gana y no tiene vuelta atrás (D32).
- Cómo se resuelve el hueco del administrador único (sección 9).
- ¿Cuenta con contraseña o solo con código por email?
- ¿Una biblioteca en la nube también se sincroniza por wifi? (D39)

## 14. Prioridades (propuesta para discutir después de los catálogos)

0. **Validar la app actual en Android** y cerrar el diseño de interfaz pendiente.
1. **El modelo de bibliotecas en la app**, sin cuenta: bibliotecas, personas, compartir por biblioteca.
2. **Roles, peticiones y firmas** entre dispositivos.
3. **Catálogos** que de verdad se van a usar (según la revisión pendiente).
4. **La nube**: servidor Symfony como un dispositivo más, registro con email.
5. **Tienda de Android** y landing.
6. Más adelante: organizaciones, panel de administración, web para el personal y servicio de pago.
