# Encargo de diseño: la web de Kosmotheka

> Preparado el 02/10/2026 para Claude Design. Se sube como guía al proyecto de Design (`guidelines/encargo-web.md`) y se pega tal cual lo que va en el bloque. Sustituye a las [páginas provisionales](../publicacion/provisional-pages/), cuyo texto de privacidad se reutiliza. Se basa en D22 (sigue abierta: la web es estática por ahora y se puede mover después), D74 (descargas), D79 (dominios) y en el [claim](../publicacion/claim.md).

```
Encargo: la web de Kosmotheka (kosmotheka.com)

Qué queremos
Una web que dé ganas de probar la app. Cercana y familiar, con personalidad y bien trabajada en contenido: que se entienda en diez segundos qué es, para quién, y por qué es distinta. Ni web corporativa ni festival de luces: cálida, clara y con algún detalle que sorprenda. Sorprendednos, con criterio.
La app aún no está publicada: Android está en pruebas y el escritorio llega pronto. La web lo dice con naturalidad ("Próximamente"), sin parecer a medio hacer.

Marca (ya en este proyecto)
Símbolo 2B, logotipo, Figtree, añil #4b53c2 (claro #8d9df5 en oscuro), tinta #0a0a0a, papel #ffffff, y las pantallas de la app de las fases 3 a 7 para las capturas. Claro y oscuro según el sistema, con conmutador.

Para quién
Familias y grupos (un club de rol, unos amigos, una asociación) con libros, cómics, juegos de mesa, películas, series, videojuegos o música en casa. Personas a las que les gusta tener sus cosas ordenadas, pero no necesariamente técnicas. Un segundo público pequeño: gente técnica a la que le importa la privacidad y el software libre.

Claim
EN: Everything your household owns, catalogued and shared.
ES: Todo lo que hay en casa, catalogado y compartido.
(Alternativa que también nos gusta: "Your home library, in sync" / "Tu biblioteca de casa, siempre al día".)

Lo que hace distinta a la app (el corazón de la landing)
- Funciona sin cuenta y sin internet: todo se guarda en tus dispositivos. La cuenta es opcional y solo sirve para sincronizar a distancia.
- Se comparte en casa: los móviles y ordenadores se emparejan con un código QR y se sincronizan por la wifi, sin servidores.
- Varias bibliotecas: la de la familia, la del club de rol, la tuya. Cada una con su gente.
- Permisos por persona: quien administra, quien edita y quien solo consulta y pide que se añada algo (y quien administra acepta o rechaza).
- Añadir es rápido: escanear el ISBN o el código de barras y los datos se rellenan solos; buscar por título; importar una hoja de cálculo.
- "¿Lo tengo?": escanear en la librería o la tienda antes de comprarlo repetido.
- Cada tipo con su ficha: jugadores y duración de un juego de mesa, dirección de una película, temporadas de una serie, plataforma de un videojuego. Series con sus huecos ("me falta el 4").
- Lo de cada uno: pendiente, leyendo, terminado, abandonado (o jugando, viendo…), estrellas, favoritos y "lo deseo".
- Tus datos son tuyos: sin publicidad, sin seguimiento, copias de seguridad cuando quieras. Será software libre (AGPL) y podrás montar tu propio servidor.
Las preguntas de la ficha de Google Play sirven de gancho: "¿Ese libro lo tenemos? ¿Dónde está el Catán? ¿A quién le presté la serie?"

Páginas
1. Inicio (la landing): claim y una imagen fuerte de la app (móvil y escritorio); el problema en pocas palabras; lo que la hace distinta; cómo se comparte en casa (el QR, la wifi, sin servidores: explicado para no técnicos, quizá con una pequeña ilustración o animación); los tipos de colección; capturas; privacidad; llamada final a descargar.
2. Descargas: Android (Google Play, "Próximamente" hasta que esté pública), Windows, Linux y, más adelante, la nube. Cada descarga dice qué sistemas admite: Windows 10 y 11; Linux .deb/.rpm/AppImage para Ubuntu 22.04 o posterior, Debian 12, Linux Mint 21 o posterior y parecidas; Ubuntu 20.04 y cualquier Ubuntu, por la Snap Store (próximamente); más distribuciones con Flathub (más adelante). Todos los botones en "Próximamente" por ahora, sin parecer rotos. Un enlace a "Versiones anteriores".
3. Cómo funciona (o "Preguntas frecuentes", decidid vosotros): ¿necesito cuenta?, ¿dónde están mis datos?, ¿qué pasa si pierdo el móvil?, ¿cómo comparto con mi familia?, ¿y con alguien de otra ciudad? (la nube, opcional, próximamente), ¿cuánto cuesta? (gratis), ¿es software libre?
4. Privacidad: la política que ya existe (os la pegamos aparte), con el mismo estilo.
Si veis sentido a otra página (por ejemplo "Sobre el proyecto": hecho por una persona, por gusto, en abierto), proponedla.

Pie
Iconos de X (@kosmotheka) y Bluesky (@kosmotheka.bsky.social), y un mailto a info@kosmotheka.com. Enlace a la privacidad. "© 2026 Kosmotheka". Selector de idioma.

Movimiento
Alguna animación al navegar, con gusto: aparecer al hacer scroll, transiciones suaves, quizá el símbolo (la estrella que se deshace en píxeles) cobrando vida en algún sitio. Nada que distraiga ni que vaya lento en un móvil viejo. Todo se apaga con prefers-reduced-motion.

Restricciones técnicas (importantes)
- Estática: HTML, CSS y JS ligero, sin frameworks ni build. Se sirve copiando una carpeta.
- Nada de terceros: ni fuentes de Google, ni CDN, ni analítica, ni cookies. Figtree en woff2 junto a las páginas. Así no hace falta aviso de cookies.
- Español e inglés: dos carpetas o dos juegos de páginas (/es/ y /en/), con el idioma del navegador como sugerencia y el selector en el pie.
- Móvil primero, accesible (contraste, foco visible, textos alternativos), rápida.
- Las capturas de la app, de vuestras pantallas, en claro y oscuro.

Entregad
Las páginas en móvil y escritorio, claro y oscuro, en los dos idiomas, y la lista de lo que hayáis supuesto.
```

## Después

- Al descargarla, va en una carpeta propia del repositorio (por decidir: `web/` en la raíz), y sustituye a `docs/publicacion/provisional-pages/` en el servidor.
- La política de privacidad se pega aparte desde [politica-privacidad.md](../publicacion/politica-privacidad.md).
- Los usuarios de X (`@kosmotheka`) y Bluesky (`@kosmotheka.bsky.social`) están confirmados (02/10/2026).
