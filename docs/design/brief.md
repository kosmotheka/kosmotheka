# Brief de diseño: Library (nombre provisional)

> Para Claude Design (o cualquier diseñador). Describe la app tal como es hoy (fin de la Fase 2), lo que no funciona bien, sobre todo en el móvil, y lo que pedimos. Acompaña a este documento el [catálogo de errores](errors.md) y las capturas de `current/`.

## 1. Qué es

Un catálogo de la biblioteca de casa, compartido por la familia: qué libros tenemos, dónde están, de quién son y en qué formato. Funciona **sin conexión y sin cuentas**: los datos viven en cada dispositivo, y los dispositivos de la familia se sincronizan por la wifi de casa. Más adelante habrá juegos de mesa, cómics y películas, préstamos y un servidor opcional.

**Uso estrella:** en una librería, sin cobertura, escaneo el código de barras de un libro y sé al momento si ya lo tenemos, de quién es y en qué formato. Si no lo tenemos, lo apunto en la lista de deseos.

## 2. Quién la usa

| Persona | Qué hace | Dónde |
|---|---|---|
| **El que organiza** (el autor: muchas novelas y libros de rol) | Carga inicial de cientos de libros, corrige datos, empareja dispositivos, hace copias | Escritorio, a ratos largos |
| **La familia** (pareja, hijo) | Consulta si algo está, dónde está; apunta deseos | Sobre todo el móvil, en momentos cortos |
| **Cualquiera, en una tienda** | «¿Lo tengo ya?» con una mano, a veces sin cobertura | Móvil, de pie, con prisa |

No son técnicos: nada de jerga («operación», «HLC», «certificado», «vector»). Sí saben qué es un ISBN o un código de barras.

## 3. Principios

1. **Sencilla primero.** Cada pantalla, una tarea principal y obvia. Lo avanzado, escondido pero accesible.
2. **Pensada para el móvil primero**, sin descuidar el escritorio, donde se hace la carga masiva.
3. **Sin conexión no es un error.** La app funciona sin red; solo algunos extras (buscar datos online) la necesitan, y lo dice con calma.
4. **Cada error explica qué pasó, por qué y qué hacer**, con un botón para hacerlo cuando sea posible. Ver la sección 7 y el [catálogo de errores](errors.md).
5. **Nada se pierde por un toque.** Borrar pide confirmación; lo demás se puede editar después.
6. **Bilingüe** (español e inglés): los textos en español son ~30 % más largos; el diseño debe aguantarlo.
7. **Accesible:** contraste AA, objetivos táctiles de 44 px como mínimo, todo usable con teclado en el escritorio, lectores de pantalla (etiquetas en botones de icono).
8. **Claro y oscuro**, siguiendo el sistema.

## 4. Plataformas y restricciones técnicas

- **App de escritorio (Linux, Windows) y Android**, construida con Tauri: la interfaz es web (React + TypeScript) dentro del navegador del sistema. No es una web pública.
- **Sistema de componentes:** [shadcn/ui](https://ui.shadcn.com) (estilo `radix-nova`, color base neutro) sobre Tailwind CSS 4, iconos [Lucide](https://lucide.dev), tipografía **Geist** (empaquetada, sin fuentes externas). Los diseños deben poder construirse con estas piezas; si hacen falta componentes nuevos, que sigan su estética.
- **Tamaños:** móvil en vertical de 360 a 430 px de ancho (con zonas seguras de Android arriba y abajo); escritorio desde 1024 px. Una tableta puede usar la versión de escritorio.
- **Imágenes:** portadas de libro (proporción ~2:3), a veces ausentes: hace falta un marcador de posición digno.
- **Cámara:** el escáner abre una vista nativa a pantalla completa; la app solo decide qué pasa antes y después.
- **Idioma:** todos los textos van en ficheros de traducción; no puede haber texto dentro de imágenes.

## 5. La app hoy y sus problemas

Hoy todo vive en **una sola pantalla** (catálogo) con una cabecera llena de botones, y el resto son **diálogos modales**. En el escritorio funciona bien; en el móvil cojea.

### Inventario actual

| Pantalla o diálogo | Qué hace |
|---|---|
| **Catálogo** (principal) | Pestañas «Catálogo / Deseos». Buscador de texto (título, autor, editorial, ISBN; sin acentos), filtros por formato, propietario y ubicación. En escritorio, tabla (portada, título, autores, año, formatos, propietarios, ejemplares); en móvil, tarjetas |
| **Cabecera** | Título y contador de libros; idioma EN/ES; Dispositivos; Ajustes; «¿Lo tengo?»; «Alta rápida»; «Añadir libro» |
| **Añadir libro** | ISBN arriba con «Buscar» (rellena datos y portada desde Google Books y Open Library); título, subtítulo, autores (varios, con sugerencias), editorial (con sugerencias), año, idioma; sección «Tu ejemplar»: propietario, formato, unidades, fecha de compra, ubicación |
| **Ficha del libro** | Pestañas: Ficha (portada, datos, editar, enlaces a Google Books y Open Library, borrar libro), Ejemplares (lista, añadir, editar y borrar ejemplar), Historial (quién cambió qué y cuándo) |
| **Alta rápida** | Datos de ejemplar para la sesión (propietario, formato, ubicación) y un campo de ISBN que encadena: escaneas o tecleas, Intro, siguiente. Lista de resultados: añadido, ya estaba (con «añadir otro ejemplar igualmente»), no encontrado (con campo para el título), error |
| **¿Lo tengo ya?** | Botón de escanear (solo móvil) y campo de ISBN. Resultado: «Sí, lo tenéis» (portada, formatos, propietarios) o «No está en vuestro catálogo» (con datos online si hay red), «Añadir a deseos», «Lo he comprado: añadir al catálogo» |
| **Deseos** | Tarjetas con portada, título, autor, ISBN y nota; «Comprado: añadir al catálogo»; quitar |
| **Dispositivos** | Nombre de este dispositivo; «Mostrar código de emparejamiento» (QR y texto copiable, vale 10 minutos y una vez); «Escanear el código del otro dispositivo» (móvil); pegar un código; lista de emparejados con «Sincronizar ahora», última sincronización, olvidar |
| **Ajustes** | Clave de API de Google Books (opcional); sección «Tus datos»: guardar copia de seguridad (.zip), restaurar copia, exportar e importar CSV |

La sincronización ocurre sola al abrir la app y al volver a ella; hoy **no se ve** en ningún sitio si está sincronizando, si falló o cuándo fue la última vez (salvo dentro de Dispositivos).

### Problemas detectados

**En el móvil:**
- La cabecera acumula siete controles: no caben con sentido, y lo más importante en el móvil («¿Lo tengo?») no destaca.
- Los diálogos modales largos (Añadir libro, Ficha) son incómodos en pantalla pequeña; deberían ser pantallas completas u hojas (*sheets*) con botón atrás.
- No hay navegación clara entre Catálogo, Deseos, Dispositivos y Ajustes.
- El formulario de alta es largo para hacerlo con una mano.
- Escanear está escondido dentro de un diálogo.

**En general:**
- Los errores dicen qué pasó, pero casi nunca **qué hacer** (ejemplo: si el móvil no empareja, el usuario no sabe qué revisar). Algunos muestran texto técnico en inglés.
- No hay indicador de sincronización ni de «sin conexión».
- Idioma y Ajustes compiten en la cabecera con acciones principales.
- Los estados vacíos existen, pero son pobres (texto y poco más).
- Ajustes mezcla una clave técnica con las copias de seguridad.

## 6. Qué proponemos (para que el diseño lo valide o lo mejore)

### Navegación

**Móvil:** barra inferior con cuatro destinos, y la acción de escanear como protagonista:

| Catálogo | **¿Lo tengo?** (botón central destacado, abre la cámara directamente) | Deseos | Más |
|---|---|---|---|

- **Más:** Añadir libro, Alta rápida, Dispositivos, Tus datos (copias y CSV), Ajustes (idioma, clave de Google, acerca de).
- Formularios y fichas como **pantallas completas** con «atrás», no como modales.
- «Añadir libro» como botón flotante en Catálogo.

**Escritorio:** barra lateral (Catálogo, Deseos, Dispositivos, Tus datos, Ajustes) o cabecera ordenada con acciones principales a la derecha («Añadir libro», «Alta rápida», «¿Lo tengo?»). Las fichas pueden seguir como paneles o diálogos anchos.

### Indicador de sincronización

Discreto y siempre visible: «Sincronizado hace 2 min», «Sincronizando…», «Sin conexión con el portátil» (con acceso a la ayuda). Tocándolo se va a Dispositivos.

### Alta de libro en el móvil

Por pasos: 1) escanear o teclear el ISBN; 2) ver lo encontrado (portada, título, autor) y confirmar o corregir; 3) datos del ejemplar con valores recordados (último propietario, formato y ubicación). Lo demás, en «Más datos».

### Varias bibliotecas (D46)

Un usuario puede tener varias bibliotecas (por ejemplo, la de la familia y una con amigos). Se trabaja con **una a la vez**, elegida con un selector siempre accesible; catálogo, deseos y altas son de la activa. El buscador ofrece «en todas» y «¿Lo tengo?» mira siempre en todas, diciendo en cuál está cada resultado. Hace falta también «Mover a otra biblioteca» en la ficha.

## 7. Errores, avisos y ayuda (muy importante)

**Patrón para todo mensaje de problema:**
1. **Qué ha pasado**, en una frase sin jerga.
2. **Por qué suele pasar**, si hay una causa probable.
3. **Qué hacer**: pasos concretos, numerados si son varios.
4. **Un botón** para la acción más probable (reintentar, abrir ajustes del sistema, mostrar un código nuevo…).
5. Opcional y plegado: **detalles técnicos** para copiar si hay que pedir ayuda.

**Niveles visuales:**
- **Información** (neutral): «Sin conexión: los datos online no están disponibles. Todo lo demás funciona.»
- **Aviso** (ámbar): algo no salió del todo, pero no se ha perdido nada.
- **Error** (rojo): la acción no se hizo; hace falta hacer algo.
- **Éxito** (verde, breve): confirmación de lo hecho.

**Dónde:** los errores de un campo, debajo del campo; los de una acción, junto a la acción o como panel en la propia pantalla; los avisos no bloqueantes, como notificación temporal (*toast*). Los problemas persistentes (emparejamiento, sincronización) tienen una **pantalla de ayuda** con comprobaciones paso a paso.

El texto completo de cada error, con causas y pasos, está en el [catálogo de errores](errors.md), incluida la guía **«El móvil no se empareja»**.

## 8. Qué pedimos

1. **Pantallas en móvil (390 × 844) y escritorio (1280 × 800), en claro y oscuro:**
   - Catálogo: con libros, vacío, buscando sin resultados.
   - Ficha del libro: vista, editando, ejemplares, historial, confirmación de borrado.
   - Añadir libro: con búsqueda por ISBN (encontrado, no encontrado, sin conexión).
   - Alta rápida: sesión en marcha con los distintos resultados.
   - ¿Lo tengo ya?: lo tienes, no lo tienes (con y sin conexión), ya en deseos, permiso de cámara denegado.
   - Deseos: con elementos y vacía.
   - Dispositivos: sin emparejar, mostrando código y QR, emparejado, error de emparejamiento con ayuda.
   - Tus datos y Ajustes.
2. **Componentes nuevos o revisados:** navegación (barra inferior y lateral), indicador de sincronización, panel de error con pasos, *toasts*, estados vacíos con ilustración o icono, marcador de portada, hoja o pantalla completa para formularios en móvil.
3. **Tokens:** colores (incluidos los de éxito, aviso y error en claro y oscuro), radios, espaciado y tipografía, compatibles con las variables CSS de shadcn/ui.
4. **Formato de entrega:** HTML y CSS con clases de Tailwind o componentes de shadcn/ui, o especificaciones claras que se puedan trasladar a ellos.

## 9. Fuera de alcance (por ahora)

Cuentas de usuario, servidor, roles y permisos (Fase 3); préstamos, estados de lectura y estadísticas (Fase 4); juegos, cómics y películas (Fase 6). El diseño no debe cerrar la puerta a que existan: por ejemplo, la navegación debe admitir más secciones.
