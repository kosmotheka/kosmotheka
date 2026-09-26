# Brief de diseño: Library (nombre provisional)

> Para Claude Design (o cualquier diseñador). Describe el producto tal como quedó decidido el 25/09/2026 (D54–D63 en [DECISIONS.md](../DECISIONS.md)) y pide el diseño de la navegación y las pantallas principales **antes de construirlas** (Fase 3 del [ROADMAP](../ROADMAP.md)). Lo acompañan el [catálogo de errores](errors.md) y una captura de la app actual en `current/`. El brief anterior está en el [histórico](../historico/brief-diseno-fases-1-2.md).

## 1. Qué es

Una app libre y gratuita para catalogar lo que tienes en casa (libros, cómics, juegos de mesa, películas y series, videojuegos, música y cualquier otra cosa) organizado en **bibliotecas** que compartes con quien quieres, con permisos.

Funciona **sin cuenta**: los datos viven en tus dispositivos y se comparten por la wifi de casa. **Con cuenta** (un email), todo se guarda también en la nube y se comparte a distancia.

**Usos estrella:**
- En una tienda, sin cobertura, escaneo algo y sé al momento si ya lo tengo, en qué biblioteca, de quién es y en qué formato.
- Apunto lo que me falta de una saga o lo que quiero que me regalen, con una foto.
- Mi hijo ve la biblioteca familiar, marca lo que ha leído y me pide añadir un libro; yo lo apruebo.

## 2. Quién la usa

| Persona | Qué hace | Dónde |
|---|---|---|
| **El que organiza** (el autor: juegos de rol, novelas, juegos de mesa, películas) | Crea bibliotecas, carga cientos de cosas, comparte y da permisos, aprueba peticiones | Escritorio a ratos largos; móvil siempre encima |
| **La familia** (pareja, hijo adolescente) | Consulta, marca lo leído, apunta deseos, pide añadir o quitar | Sobre todo el móvil, en momentos cortos |
| **Amigos** | Reciben una biblioteca compartida; a veces la editan juntos | Móvil |
| **Cualquiera, en una tienda** | «¿Lo tengo?» con una mano, a veces sin cobertura | Móvil, de pie, con prisa |

No son técnicos: nada de jerga («operación», «sincronizar vectores», «firma», «certificado»). Sí saben qué es un código de barras o un ISBN.

## 3. Principios

1. **Sencilla primero.** Cada pantalla, una tarea principal y obvia. Lo avanzado, escondido pero accesible. Nada que suene a corporativo.
2. **Una app de Android de verdad, no una web metida en una ventana.** Seguir los patrones de la plataforma todo lo posible: barra de navegación inferior, menú lateral (*navigation drawer*), barra superior, botón atrás del sistema, hojas inferiores, botón flotante, gestos y zonas seguras. En escritorio, los patrones de una app de escritorio.
3. **Móvil primero**, sin descuidar el escritorio, donde se hace la carga masiva.
4. **Sin conexión no es un error.** Todo funciona sin red; solo algunos extras (buscar datos online, la nube) la necesitan, y lo dicen con calma.
5. **Sin cuenta es de primera**, no una versión recortada. La cuenta se ofrece, nunca se impone.
6. **Cada problema explica qué pasó, por qué y qué hacer**, con un botón para hacerlo (sección 9 y [catálogo de errores](errors.md)).
7. **Nada se pierde por un toque.** Borrar pide confirmación; lo que un rol no puede hacer se **pide**, no se prohíbe en seco.
8. **Bilingüe** (español e inglés): los textos en español son ~30 % más largos.
9. **Accesible:** contraste AA, objetivos táctiles de 44 px como mínimo, teclado en escritorio, etiquetas en botones de icono.
10. **Claro y oscuro**, siguiendo el sistema.

## 4. Plataformas y restricciones técnicas

- **Android, Linux y Windows**, con Tauri: la interfaz es web (React + TypeScript) dentro del navegador del sistema. **iOS no**, y no es una web pública.
- **Componentes:** [shadcn/ui](https://ui.shadcn.com) (estilo `radix-nova`, base neutra) sobre Tailwind CSS 4, iconos [Lucide](https://lucide.dev), tipografía **Figtree** (licencia SIL OFL 1.1), empaquetada con la app para que funcione sin red; sustituye a Geist desde el 27/09/2026. Los diseños deben poder construirse con estas piezas; lo que falte (barra inferior, menú lateral, hojas), con su misma estética.
- **Marca** (desde el 27/09/2026; piezas y reglas de uso en [`brand/`](../../brand/README.md)):
  - **Símbolo:** una estrella calada en un lienzo redondeado que se deshace en una escalera de píxeles hacia la esquina de abajo a la izquierda. Logotipo «Kosmotheka» en Figtree SemiBold. En la barra lateral de escritorio va la combinación horizontal.
  - **Color de acento: añil.** En claro `#4b53c2` · `oklch(0.50 0.17 275)`, con 6,3:1 sobre blanco. En oscuro `#8d9df5` · `oklch(0.72 0.13 275)`, con 7,8:1 sobre `#0a0a0a`. Solo como acento (botón principal, enlaces, foco y selección) sobre la interfaz neutra, nunca de fondo de pantallas enteras. No se confunde con los colores de los estados (verde, ámbar, rojo, rosa).
- **Tamaños:** móvil vertical de 360 a 430 px de ancho, con zonas seguras de Android; escritorio desde 1024 px.
- **Imágenes:** portadas y cajas de proporciones distintas (libro ~2:3, caja de juego ~1:1, carátula de película ~2:3, videojuego ~3:4), a veces ausentes: hace falta un marcador digno por tipo.
- **Cámara:** el escáner abre una vista nativa a pantalla completa; la app decide qué pasa antes y después.
- **Textos:** todos en ficheros de traducción; ningún texto dentro de imágenes.

## 5. El modelo, en palabras del usuario

| Concepto | Qué es para el usuario |
|---|---|
| **Biblioteca** | Una caja que compartes: «Rol», «Familia», «Videojuegos de Hugo». Un dispositivo puede tener varias; se trabaja con **una a la vez** |
| **Obra** | Lo que catalogas: un libro, un cómic, un juego de mesa, una película o serie, un videojuego, u **«Otro»** (sellos, monedas, Playmobil) |
| **Ejemplar** | Lo que tienes de verdad: en qué formato (tapa dura, PDF, Blu-ray, Steam…), de quién es y dónde está. Lo digital no tiene estantería («Steam», «Kindle») |
| **Serie** | Agrupa obras: una saga con orden (Dune 1–6) o una línea sin orden (todas las versiones de Catán). Una obra puede estar en varias |
| **Estado** | **Lo tengo** (tiene ejemplares; se ve por defecto) · **Me falta** (está apuntada, sin ejemplares) · **Lo deseo** (una marca para que te la regalen) |
| **Persona** | Quien usa una biblioteca: tú, tu pareja, Hugo. Tiene sus dispositivos, su rol, sus lecturas y sus deseos |
| **Rol** | **Administrador** (todo, incluido dar permisos y aprobar), **editor** (crea y edita; borra solo lo suyo), **lector** (ve, marca lo suyo y pide) |
| **Petición** | Lo que un rol no puede hacer se pide («añade este libro», «quita este»), y un administrador lo aprueba o lo rechaza |
| **Compartir** | Llevar una biblioteca a otro dispositivo, diciendo de quién es y con qué rol. Un permiso aparte: quien no lo tiene puede **enseñar** su pantalla, no compartir |
| **Nube** | Opcional: con un email, tus bibliotecas se guardan fuera y se comparten a distancia |

## 6. Navegación (propuesta para validar o mejorar)

### Móvil

**Barra superior:** a la izquierda, el botón del menú lateral. En el centro, el nombre de la biblioteca activa, que es el **selector**: al tocarlo se abre una hoja con tus bibliotecas y «Nueva biblioteca». A la derecha, buscar y, **solo para administradores**, una **campanita con el contador de peticiones** pendientes, que abre la bandeja.

**Barra inferior, tres destinos**, simétrica:

| Catálogo | **Escanear / ¿Lo tengo?** (acción central destacada) | Me falta |
|---|---|---|

- **Catálogo:** lo que tienes en la biblioteca activa, con filtros por tipo, formato, persona, ubicación y serie. Botón flotante «Añadir».
- **Escanear:** abre la cámara directamente; el resultado busca **en todas** tus bibliotecas.
- **Me falta:** lo apuntado sin ejemplares, con la marca de deseo; un filtro «solo deseos» es la lista de regalos.

**Peticiones:** el administrador las recibe por la campanita. Quien no administra no ve campanita: sus peticiones («Mis peticiones», con su estado) están en el menú lateral.

**Menú lateral:** bibliotecas; mis peticiones; compartir y dispositivos; personas y permisos; series; copias de seguridad; cuenta y nube; ajustes (idioma, proveedores de datos, clave de Google Books); **ayuda**; acerca de.

**Indicador de sincronización**, discreto y siempre visible: «Al día», «Sincronizando…», «Sin conexión con la nube», con acceso a la ayuda.

### Escritorio

Barra lateral con las mismas secciones y el selector de biblioteca arriba; acciones principales («Añadir», «Alta rápida», «Escanear» si hay cámara) en la barra superior. Las fichas pueden ser paneles o diálogos anchos.

## 7. Flujos principales

### Añadir algo

1. **Empieza por buscar o escanear**, no por un campo concreto: un campo de búsqueda (título, autor, código) y un botón de cámara.
2. Si se sabe el **tipo** (el código de un libro lo dice), se elige solo; si no, el usuario lo elige (libro, cómic, juego de mesa, película o serie, videojuego, otro).
3. **Resultados de los proveedores** con portada, título y datos clave; al elegir uno se rellena la ficha.
4. **Siempre hay «Añadir a mano»**, visible y sin culpa, aunque no haya red ni resultados.
5. **Datos del ejemplar** con valores recordados (último formato, persona y ubicación), o «No lo tengo: apuntar como me falta» (con la opción de marcarlo como deseo).
6. Si la obra ya existe en esa biblioteca, se ofrece añadir un ejemplar en vez de duplicarla.

El formulario es **el mismo para todos los tipos**: una parte común (título, portada, año, series, notas) y una parte específica del tipo (ISBN y autores en libros; plataforma en videojuegos; jugadores en juegos de mesa…). «Otro» es solo la parte común.

**Proveedores:** sin cuenta solo hay algunos (libros, música, y juegos de mesa por código). Para películas, videojuegos o cómics sin cuenta, la búsqueda lo dice con calma («Para buscar películas necesitas la nube; puedes añadirla a mano») y enlaza a la ayuda.

### ¿Lo tengo?

Escanear o teclear. Resultado: **«Lo tienes»** (en qué bibliotecas, en qué formato, de quién) · **«Te falta»** (apuntado, quizá como deseo) · **«No lo tienes»** (con datos online si hay red) con «Apuntar como me falta», «Marcar como deseo» o «Lo he comprado: añadir».

### Series

Vista de una serie con sus obras en orden y **los huecos** (lo que falta) bien visibles; añadir un hueco desde la propia serie. En la ficha de una obra, sus series. Relación «necesita» (una expansión necesita el básico) visible en la ficha.

### Compartir una biblioteca

Desde la biblioteca: «Compartir» → mostrar un QR (o escanear el del otro) → **decir de quién es ese dispositivo** (una persona existente o nueva) y **con qué rol**. Fuera de casa, la ayuda explica el truco del punto de acceso del móvil. Con cuenta, compartir por invitación a distancia.

### Personas y permisos

Lista de personas de la biblioteca con su rol y sus dispositivos; cambiar rol, dar o quitar «puede compartir», revocar un dispositivo perdido. Aviso cuando solo hay un administrador: «Si pierdes este móvil, nadie podrá gestionar la biblioteca. ¿Quieres que otra persona también administre?».

### Peticiones

El lector ve «Pedir que se añada» y «Pedir que se quite» donde otros ven «Añadir» y «Borrar». Las peticiones muestran quién, qué y cuándo; el administrador acepta o rechaza con un toque, y el lector ve el resultado.

### Cuenta y nube

Un botón «Guardar en la nube» (en el menú y en momentos oportunos, nunca insistente): email → código recibido por correo → listo, se sube lo que hay. Estado de la nube y qué bibliotecas están en ella.

### Copias de seguridad

**Muy fácil y a la vista:** «Guardar copia» de todo o de una biblioteca, al sitio que elija el usuario (Google Drive…). Restaurar, exportar e importar CSV.

## 8. Ayuda

Una sección propia en el menú, con textos cortos:
- Qué se puede hacer sin cuenta y qué añade la nube.
- Por qué algunos buscadores solo funcionan con la nube.
- Cómo compartir en casa, fuera de casa (punto de acceso) y a distancia.
- Roles y peticiones.
- Qué hacer si pierdes el móvil.
- «El móvil no se empareja» ([catálogo de errores](errors.md)).

## 9. Errores, avisos y ayuda (muy importante)

**Patrón para todo mensaje de problema:**
1. **Qué ha pasado**, en una frase sin jerga.
2. **Por qué suele pasar**, si hay una causa probable.
3. **Qué hacer**: pasos concretos, numerados si son varios.
4. **Un botón** para la acción más probable.
5. Opcional y plegado: **detalles técnicos** para copiar si hay que pedir ayuda.

**Niveles:** información (neutral) · aviso (ámbar, no se perdió nada) · error (rojo, la acción no se hizo) · éxito (verde, breve).

**Dónde:** los de un campo, debajo del campo; los de una acción, junto a ella; los no bloqueantes, como *toast*; los persistentes (compartir, sincronizar), con una pantalla de ayuda paso a paso.

Casos nuevos a contemplar: un cambio tuyo que quedó como petición porque perdiste el permiso sin conexión; un dispositivo que dejó de estar autorizado; un proveedor que necesita la nube.

## 10. Qué pedimos

1. **Navegación** en móvil (390 × 844) y escritorio (1280 × 800), claro y oscuro: barra superior con selector de biblioteca y campanita de peticiones, barra inferior de tres destinos, menú lateral, indicador de sincronización.
2. **Pantallas en móvil**, y en escritorio las marcadas con (E):
   - Catálogo (E): con elementos de varios tipos, vacío, filtrado sin resultados.
   - Selector de bibliotecas y «Nueva biblioteca».
   - Añadir: buscar, resultados, sin resultados, sin red, proveedor que necesita la nube, a mano, elegir tipo (E).
   - Ficha de una obra de cada tipo (E): vista, editando, ejemplares, series, historial.
   - ¿Lo tengo?: lo tienes (en varias bibliotecas), te falta, no lo tienes (con y sin red), permiso de cámara denegado.
   - Me falta y deseos.
   - Serie con huecos (E).
   - Compartir: mostrar QR, escanear, asignar persona y rol, error de emparejamiento con ayuda.
   - Personas y permisos, con el aviso de administrador único.
   - Peticiones: campanita con contador y bandeja del administrador; «Mis peticiones» del lector en el menú.
   - Cuenta y nube: registro con código, estado.
   - Copias de seguridad; Ajustes; Ayuda.
3. **Componentes:** barra inferior, menú lateral, campanita con contador, selector de biblioteca en hoja, indicador de sincronización, tarjeta de obra por tipo, marcador de portada por tipo, insignias de estado (lo tengo / me falta / deseo), panel de error con pasos, *toasts*, estados vacíos, hoja o pantalla completa para formularios.
4. **Tokens:** colores (con éxito, aviso y error en claro y oscuro), radios, espaciado y tipografía, compatibles con las variables CSS de shadcn/ui.
5. **Formato de entrega:** HTML y CSS con Tailwind o componentes de shadcn/ui, o especificaciones claras que se puedan trasladar a ellos.

## 11. Fuera de alcance (por ahora)

Préstamos, reserva de regalos y estadísticas (Fase 9); inventario y foto de balda (Fase 10); organizaciones, panel de administración y web para el personal (Fase 11). La navegación no debe cerrarles la puerta: tiene que admitir más secciones.
