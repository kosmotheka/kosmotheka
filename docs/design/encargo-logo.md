# Encargo de diseño: logo de Kosmotheka

> Tercer encargo para Claude Design, después del [brief](brief.md) y del [encargo de la Fase 4](encargo-fase-4.md). El nombre está decidido en D70 ([DECISIONS.md](../DECISIONS.md)). Hace falta para Google Play (Fase 8), para la web (landing y servidor en la nube) y para la propia app.

## Qué es un logo aquí: un sistema, no una imagen

Las apps conocidas no usan un logo distinto en la tienda y en la web: usan **un símbolo** y lo combinan de varias formas. Spotify, por ejemplo, tiene el «logo completo» (símbolo + nombre) como versión principal y el «solo icono» para donde no cabe, como la tienda de apps; el nombre nunca aparece sin el símbolo. Lo mismo hacen Firefox, Duolingo o Notion.

Por eso pedimos:

1. **Un símbolo** (isotipo) que funcione solo, sin letras. Es el icono de la app, el de la tienda y el favicon.
2. **El nombre escrito** (logotipo) «Kosmotheka».
3. **Las combinaciones** de los dos (horizontal y apilada) para la web, la cabecera de la app y los documentos.

Lo que cambia entre plataformas no es el dibujo, sino **el marco** (cuadrado a sangre, capas adaptativas, margen de seguridad) y, en tamaños muy pequeños, **el detalle** (un favicon de 16 px puede necesitar una versión simplificada del símbolo).

**Resultado (27/09/2026):** símbolo 2B, color añil y Figtree. Piezas en [`brand/`](../../brand/README.md); la búsqueda de marcas está en [research/logo.md](../research/logo.md).

## El nombre

*Kosmotheka*: griego *kosmos* (el mundo **ordenado**, lo contrario del caos) y *theka* (caja, depósito, como en biblioteca o pinacoteca). **El cosmos de tus cosas.**

- No es solo para libros: sirve para cómics, juegos de mesa, películas, videojuegos y cualquier colección. **Evitar un libro como símbolo** (o un libro solo): encasilla la app.
- Ideas que pueden inspirar, sin obligar: orden, estantería, constelación (cosas sueltas que forman una figura), caja o contenedor, órbitas, la K.
- El tono: cercano, familiar, libre y gratuito. **Nada corporativo ni frío**, nada de «tecnología». Lo usan una familia y sus amigos (ver [brief](brief.md) §2 y §3).

## Restricciones

- **Legible a 16 px** (favicon) y reconocible a 48 dp (icono del móvil). Probarlo a esos tamaños, no solo en grande.
- **Funciona en una sola tinta** (negro o blanco sobre cualquier fondo): Android 13+ tiñe el icono con los colores del fondo de pantalla usando solo su silueta.
- **Claro y oscuro.**
- **Color de marca:** la interfaz es neutra (shadcn/ui, base gris, ver `templates/`). El logo puede traer **un color propio**, que después usaremos como acento en la app. Proponerlo con su contraste AA sobre blanco y sobre el fondo oscuro de la app.
- **Tipografía del logotipo:** una fuente con licencia libre (SIL OFL o similar), o letras dibujadas a mano. En el SVG final, el texto va **convertido a trazos**. Decir qué fuente se usó.
- **Sin sombras, degradados complejos ni texturas** en el icono: Google Play añade la sombra y redondea las esquinas por su cuenta.
- **Original:** que no se parezca a logos conocidos de catálogos o bibliotecas (Goodreads, Libib, BoardGameGeek, Letterboxd…). Vamos a buscarlo en los registros de marcas antes de publicarlo.

## Qué entregar

Todo en **SVG**, con `viewBox` y sin imágenes incrustadas. A partir de ellos generaremos todos los PNG, ICO y demás tamaños.

### Marca

| Pieza | Detalle |
|---|---|
| Símbolo | En color, en negro y en blanco. Lienzo cuadrado, 1024 × 1024 |
| Logotipo | «Kosmotheka» en trazos, en color, negro y blanco |
| Combinación horizontal | Símbolo + logotipo, para la cabecera de la web y la barra lateral de escritorio |
| Combinación apilada | Símbolo encima del nombre, para la landing y documentos |
| Hoja de uso | Una página con el margen mínimo alrededor, el tamaño mínimo, los colores (hex y oklch) y lo que no se debe hacer |

### Icono de la app

| Pieza | Especificación |
|---|---|
| **Icono cuadrado a sangre** | 1024 × 1024, el fondo ocupa todo el cuadrado, **sin esquinas redondeadas ni sombra**. De aquí sale el icono de Google Play (512 × 512 PNG) |
| **Capa de fondo adaptativa** (Android) | Lienzo de 108 × 108 dp (en SVG, `viewBox="0 0 108 108"`). Color liso o dibujo sencillo |
| **Capa frontal adaptativa** (Android) | Mismo lienzo, fondo transparente. El símbolo **dentro del círculo central de 66 dp** (zona que nunca se recorta); lo de fuera puede cortarse o moverse. Entre 48 y 66 dp de tamaño |
| **Capa monocroma** (Android 13+) | Mismo lienzo y posición que la frontal, una sola tinta, fondo transparente |
| **Icono de escritorio** (Linux y Windows) | 1024 × 1024 con **fondo transparente** y el símbolo en su propia forma (por ejemplo, un cuadrado redondeado con ~6 % de margen). En escritorio un cuadrado a sangre queda raro |
| **Icono de la pantalla de inicio** (Android 12+) | Lienzo de 240 × 240 dp y el símbolo dentro de un círculo de 160 dp, más el color de fondo de esa pantalla, claro y oscuro |

### Web

| Pieza | Especificación |
|---|---|
| Favicon | SVG simplificado para 16–32 px, que se vea en pestañas claras y oscuras (puede llevar `@media (prefers-color-scheme: dark)` dentro) |
| Icono para instalar la web (*maskable*) | 512 × 512 a sangre, con el símbolo dentro del círculo central del 80 % |
| Imagen para compartir | 1200 × 630 (redes y enlaces) y 1280 × 640 (vista previa del repositorio en GitHub): combinación horizontal sobre fondo de marca, sin más texto |

### Google Play

| Pieza | Especificación |
|---|---|
| Gráfico destacado | 1024 × 500. Solo la marca y, como mucho, un fondo; **sin frases**, que irían traducidas y se hacen aparte |

Las **capturas de pantalla** de la tienda no entran en este encargo: se harán con la app terminada.

## Cómo lo usaremos

- **App (Tauri):** `npm run tauri icon` genera todos los tamaños de escritorio y Android desde un solo fichero. Las capas adaptativas y la monocroma se colocan aparte en el proyecto Android (`mipmap-anydpi-v26/ic_launcher.xml`), porque hoy la app solo tiene iconos PNG de un solo nivel.
- **Web:** `favicon.svg`, `favicon.ico` (16, 32 y 48), `apple-touch-icon.png` (180) y los iconos del manifiesto (192 y 512, normal y *maskable*), todos generados desde los SVG.
- Los SVG maestros vivirán en el repositorio (`brand/` o similar) para regenerar los tamaños cuando cambie algo.

## Protección de la marca (no es parte del encargo)

Propuesta en D72 ([DECISIONS.md](../DECISIONS.md)): el nombre y el logo quedan fuera de la AGPL, con una política de marca aparte. Antes de elegir el logo definitivo se busca por imagen en TMview y WIPO Global Brand Database, y el logo elegido se registra como marca figurativa.
