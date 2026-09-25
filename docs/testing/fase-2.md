# Pruebas manuales de la Fase 2

> Mismo formato que `hito-1.md`: pasos, resultado esperado, `[x]` al pasar. El motor de sincronización (2.0) y el protocolo de red (2.1) tienen tests automáticos que simulan dispositivos y conexiones TLS reales. Aquí va lo que solo se ve con la app delante.

## Preparar dos instancias en el mismo equipo

La prueba más cómoda del 2.1 no necesita el móvil: dos copias de la app en tu portátil, cada una con sus propios datos.

1. En una terminal del contenedor: `cd app && npm run tauri dev` (instancia **A**, con tus datos de siempre).
2. En otra terminal del contenedor, con la primera abierta:
   `LIBRARY_DATA_DIR=/tmp/library-b ~/Proyectos/personales/library/target/debug/library-app` (instancia **B**, vacía).
3. Las dos ventanas se llaman «Library». Para distinguirlas, pon nombres distintos en «Dispositivos» → nombre de este dispositivo.

> En la instancia B no se ven las portadas: el permiso para mostrarlas apunta a la carpeta de datos normal. Solo ocurre con `LIBRARY_DATA_DIR`, que existe únicamente en las compilaciones de desarrollo.
> Para empezar B desde cero: ciérrala y borra `/tmp/library-b`.

## 2.0 · Motor de sincronización

- [ ] **Tus libros de antes viajan.** Tras emparejar (ver 2.1), B recibe todos los libros de A, incluidos los que diste de alta antes del registro de operaciones (el de Tolkien). En su historial aparece «Incluido en la sincronización (estado inicial)».

## 2.1 · Sincronización por la red local

- [ ] **Mostrar el código.** En A → icono de dispositivos (pantalla y móvil) → «Mostrar código de emparejamiento». *Esperado:* un QR y un texto que empieza por `LIBRARY1:`, con botón para copiarlo. En Windows, el cortafuegos puede preguntar: permítelo en redes privadas.
- [ ] **Emparejar.** Copia el texto → en B → Dispositivos → pégalo → «Emparejar». *Esperado:* «Emparejado con … Recibidos N cambios, enviados 0», y la tabla de B muestra los libros de A.
- [ ] **El código solo vale una vez.** Intenta emparejar otra vez con el mismo texto (por ejemplo, desde una tercera instancia con otra carpeta). *Esperado:* «El otro dispositivo lo ha rechazado…».
- [ ] **Cambios en los dos sentidos.** Añade un libro en B → Dispositivos → «Sincronizar ahora». *Esperado:* «recibidos 0, enviados 1»; en A aparece el libro (quizá tengas que cerrar y abrir Dispositivos o cambiar el filtro para refrescar la tabla).
- [ ] **Conflicto: gana el último.** Edita el año del mismo libro en A y después en B, sincroniza. *Esperado:* en los dos queda el valor de B.
- [ ] **Campos distintos se conservan.** En A cambia el año; en B, el idioma, sincroniza. *Esperado:* en los dos quedan ambos cambios.
- [ ] **Gana el borrado.** Borra un libro en A; en B, edítalo; sincroniza. *Esperado:* el libro desaparece de los dos.
- [ ] **Duplicados.** Da de alta el mismo ISBN en A y en B antes de sincronizar. *Esperado:* tras sincronizar, el libro aparece dos veces en los dos (se fusionarán a mano más adelante); las sugerencias de autores no muestran nombres repetidos.
- [ ] **Sincronización al abrir.** Cierra B, añade algo en A, abre B. *Esperado:* B se pone al día sola al abrirse.
- [ ] **Dispositivo apagado.** Cierra A y pulsa «Sincronizar ahora» en B. *Esperado:* «No se encuentra el otro dispositivo…», sin nada roto.
- [ ] **Olvidar.** En A, olvida a B. Luego sincroniza desde B. *Esperado:* «El otro dispositivo lo ha rechazado…». Se puede volver a emparejar con un código nuevo.
- [ ] **Entre dos ordenadores de verdad.** Si tienes otro equipo en casa con la app (por ejemplo, el `.exe` de Windows de la CI), repite el emparejamiento por la wifi.

## 2.2 · Móvil, escáner y «¿lo tengo ya?»

> Instalar en el móvil: con la depuración inalámbrica activada, dentro del contenedor, `cd app && npm run tauri android dev`. También vale el APK de `app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/` (ver `docs/development/setup.md`).

- [ ] **Vista de móvil.** La app en el móvil muestra el catálogo en tarjetas (portada, título, autores, formatos), no en tabla. La cabecera cabe, con iconos sin texto.
- [ ] **Escanear.** «¿Lo tengo?» → «Escanear código de barras» → la primera vez pide permiso de cámara → apunta al código de barras de un libro. *Esperado:* se cierra la cámara y se ve la respuesta.
- [ ] **Lo tienes.** Escanea un libro que esté en el catálogo. *Esperado:* recuadro verde «Sí, lo tenéis» con portada, formatos y propietarios; tocarlo abre su ficha.
- [ ] **No lo tienes.** Escanea uno que no esté. *Esperado:* «No está en vuestro catálogo»; con conexión aparece debajo su título y portada, sin conexión no.
- [ ] **Sin cobertura.** Modo avión → escanea un libro que tengas. *Esperado:* la respuesta llega igual, al instante.
- [ ] **Apuntar un deseo.** Con un libro que no tienes → escribe o revisa el título → «Añadir a deseos». *Esperado:* «Ya está en la lista de deseos»; aparece en la pestaña «Deseos».
- [ ] **Del deseo al catálogo.** En «Deseos» → «Comprado: añadir al catálogo». *Esperado:* se abre el alta con ISBN, título y autor rellenados; al guardar, el libro está en el catálogo y el deseo desaparece.
- [ ] **Permiso denegado.** Deniega la cámara. *Esperado:* un mensaje que explica cómo activarla en los ajustes, sin cerrarse la app.
- [ ] **En el escritorio.** «¿Lo tengo?» no muestra el botón de escanear; teclear el ISBN (o usar un lector USB) + Comprobar funciona igual.
- [ ] **Los deseos se sincronizan.** Apunta un deseo en el móvil, sincroniza con el escritorio (hito 2.3 o pegando el código a mano) y comprueba que aparece allí.

