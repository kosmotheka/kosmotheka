# Nombre del proyecto

> **Elegido: Kosmotheka (26/09/2026).** «library» sigue en el código hasta que se renombre. No bloquea el desarrollo: carpetas, *crates* e identificadores se pueden renombrar. El identificador de Android solo importa de verdad al publicar en una tienda. Al cambiarlo, las APK de prueba se desinstalan y se vuelven a instalar (y los datos locales del móvil se recuperan sincronizando).

## Criterios

- **Global, no solo para hispanohablantes** (26/09/2026). La app sale en inglés y español; al ser open source, podrá conectarse a un servicio de traducción colaborativa para añadir idiomas. Los que se escriben de derecha a izquierda (árabe) se verán cuando lleguen.
- Corto y fácil de pronunciar en español y en inglés.
- Que funcione escrito en un enlace o un mensaje: se comparte con gente que no tiene la app.
- **No limitado a los libros**: también habrá juegos, cómics, música y películas.
- Libre: repo en GitHub, dominio, Play Store y marca registrada.

**Por decidir:** hay dos enfoques que tiran en direcciones contrarias.
- **Un nombre que no signifique nada evidente** (Funes): no encasilla. Es la idea de la primera ronda (24/09/2026).
- **Un nombre que diga lo que hace** (estante, balda, lomo, tejuelo, «-teca»): se entiende a la primera, pero casi todos suenan a libros y son difíciles en inglés. Es la idea de la segunda ronda (26/09/2026).

## Candidatos

**Elegido (26/09/2026): Kosmotheka** ([D70](../DECISIONS.md); plan en [plans/renombrado-kosmotheka.md](../plans/renombrado-kosmotheka.md)). Griego *kosmos* (mundo ordenado, lo contrario del caos) + *theka* (depósito): el cosmos de tus cosas. .com y .app libres el día de la elección, Google Play y marcas limpios (ver «Quinta ronda»). Los errores de escritura (Kosmoteka, Kosmotek) tienen el .com ajeno, pero hoy casi todo el mundo llega por un enlace, no tecleando. El maintainer registra kosmotheka.com y kosmotheka.app. En GitHub, `kosmotheka` estaba libre.

Pantheka queda como segunda opción.

**Pendiente:**
- En Google Play el título no es único; lo único que se reserva es el identificador del paquete, al subir la primera versión en la consola. Hoy es `com.tersoal.library`.
- Solo una marca registrada protege el nombre.

**Descartados el 26/09/2026, porque todos tienen alguna costura:** Musary («mus» en español; .com ajeno), Colleo (marcas parecidas; .com en venta), Pantheca (suena a panadería en español), Thesorium (.com registrado en 2025), Gatherium (.com con web viva y marca parecida en la clase 9) y Panteka (.com en venta y marca en la clase 28, juegos).


### Primera ronda (24/09/2026)

| Nombre | Idea | Pega | Opinión del maintainer |
|---|---|---|---|
| **Funes** | «Funes el memorioso» (Borges): el que lo recuerda todo | Guiño que no todo el mundo pilla | Le gusta: al no significar nada evidente, no encasilla |
| **Kinshelf** | *Kin* (familia) + *shelf* (estantería) | Suena solo a libros | Le gusta más, pero duda por eso |
| Balda | Donde viven libros y juegos | Muy español; balda.app está cogido | — |
| Holding | *Holdings* de biblioteca + la *Bag of Holding* de D&D | En inglés también es «sociedad holding» | — |
| Tomo | Volumen; igual en español, inglés y japonés | Suena a libros y está muy usado | — |
| Lomo | El lomo del libro, lo que lee la foto de la balda | En España, embutido; lomo.app está cogido | — |

Ninguno de estos se ha pasado todavía por TMview ni por los registros de dominios.

### Segunda ronda (26/09/2026)

Dominios consultados el 26/09/2026 en los registros oficiales (Verisign para .com, Google Registry para .app). Marcas buscadas en TMview el mismo día con un script propio: búsqueda «Contiene» en todas las oficinas (resultado de la búsqueda del 26/09/2026; los datos no se conservan en el repositorio).

| Nombre | Qué evoca | .com | .app | Marcas en TMview | Conflicto (clases 9, 41, 42) |
|---|---|---|---|---|---|
| ★ **Estanteca** | Estante + «-teca»; sirve para libros y juegos | Libre | Libre | Ninguna | No |
| ★ **Coleteca** | Colección + «-teca»; ideal si hay varios catálogos | Libre | Libre | Ninguna | No |
| ★ **Tejuelo** | La etiqueta del lomo en las bibliotecas | Cogido (agosto 2026); tejueloapp.com libre | Libre | 5 (ver abajo) | **Sí**: 41 en Argentina |
| **Anaquelia** | De «anaquel», la balda de librería | Libre | Sin comprobar | Ninguna | No |
| **Baldario** | Balda + «-ario», como herbario | Libre | Libre | 1: Egipto, clase 5 (farmacia) | No |
| **Lomoteca** | Biblioteca de lomos | Libre | Sin comprobar | 5, todas de restauración | No |
| **Mibalda** | «Mi balda» | Libre | Sin comprobar | 2 en Corea, por transliteración | No |
| **Ludobalda** | Posible marca para la ludoteca | Libre | Sin comprobar | Ninguna | No |

Otras apps con nombres parecidos: estante.app es de Estante Digital (Brasil); hay apps llamadas «Coleccionista» y «MisColecciones+».

### Marcas encontradas

**Tejuelo**
| Marca | Titular | Oficina | Estado | Clases |
|---|---|---|---|---|
| TEJUELO EDITORES (2021) | una persona física (titular) | Argentina (INPI) | Registrada | **41** (publicación de libros, también electrónica) |
| TEJUELO EDITORES (2010) | una persona física (titular) | Argentina (INPI) | Registrada | **41** |
| TEJUELOS (con logotipo) | una persona física (titular) | Perú (INDECOPI) | Solicitada | 43 |
| MONTEJUELO | una persona física (titular) | España (OEPM) | Registrada | 33 |
| NAVA DEL TEJUELO | Quintaplast La Mancha, S.L. | España (OEPM) | Solicitada | 33 |

**Baldario**: BALDARIO, de Shandong Sino-Agri United Biotechnology Co. Egipto, registrada, clase 5.

**Lomoteca**
| Marca | Titular | Oficina | Estado | Clases |
|---|---|---|---|---|
| LOMOTECA LOS CAROLINOS | una persona física (titular) | Argentina | Vencida | 35 |
| LA LOMOTECA | una persona física (titular) | Argentina | Vencida | 43 |
| PARRILLA A LA CHAPA LOMOTECA | A la Chapa SAS | Argentina | Registrada | 35 y 43 (dos marcas) |
| LA LOMOTECA ASADOR ARGENTINO… | dos personas físicas (titulares) | España (OEPM) | Registrada | 43 |

**Mibalda**: 미발단 y 봄 미발단, de 장영철. Corea del Sur, finalizadas, clase 30.

Límites: la búsqueda «Contiene» solo encuentra el texto literal, no marcas que se parezcan por sonido o forma. TMview no es un registro oficial.

### Tercera ronda (26/09/2026): colección general y global

Enfoque elegido: evocar **colección en general** (juegos, música, lo digital), sin muebles ni libros, y que funcione en inglés y en español. Dominios por RDAP (Verisign para .com, Google para .app); marcas en TMview con el script, fijándose solo en las marcas iguales al nombre.

**Palabras reales: todas cogidas.** Vitrina, vitrine, arca, kist, omnia, collecta, arkive, musea y curiosa tienen .com y .app registrados y entre 13 y 54 marcas en las clases 9, 41 o 42.

**Inventadas: el .com está cogido en todas** (29 probadas, de cinco a nueve letras). Un .com corto y pronunciable es prácticamente imposible; habrá que ir a .app o a una variante (`getX.com`, `Xapp.com`).

| Nombre | Idea | .app | Marcas en TMview | GitHub | Valoración |
|---|---|---|---|---|---|
| ★ **Musary** | *Museum* + «-ary» (como *library*, *aviary*): un museo personal | Libre | Ninguna igual ni parecida | Usuario vacío (0 repos) | **Candidato.** El más limpio; libre en Google Play. El .com es de un holandés desde 2009, sin web |
| ★ **Colleo** | Evoca *collect* | Libre | COLLEO vigente solo en inmobiliaria (36, Alemania); las de la 42 están vencidas | Usuario vacío (0 repos) | **Candidato.** Libre en Google Play. El .com está a la venta |
| Omnika | *Omni*: todo | Libre | «OMNIKA 5.7» registrada en Portugal en 41 y 42 | — | Dudoso |
| Arkeep | El arca (uno de cada) + *keep* | Libre | ARKEEP registrada en la UE en la clase 9 | — | Descartado |

**Descartados por cómo suenan** (con .app libre): Hoardia (*hoarder*, acumulador compulsivo), Cosario (suena a «corsario»), Arkadu, Colectio.

**.app cogido:** kollo, arkeo, vitrino, omnio, tenko, kisto, curato, trovi, shelvo, kepto, arkio, amasso, pilo, stashi, havly, gathero, ownbox, vitrin, collectia, omnikeep, kollecta, arkivo.

### Cuarta ronda (26/09/2026): nombres largos y potentes

El maintainer prefiere un nombre largo pero potente a uno corto: el proyecto no es un negocio y no hay marketing que pelear. Se añade un criterio: **validar en Google Play**, no solo el dominio. Con un script propio (la búsqueda de Play es aproximada, así que se miran los títulos y los ids de las apps, no si hay resultados).

| Nombre | Idea | .com | .app | Google Play | Marcas en TMview |
|---|---|---|---|---|---|
| **Pantheca** | Griego *pan* (todo) + *theca* (depósito, como *bibliotheca*): el depósito de todo | Cogido | Libre | Libre | Ninguna igual (solo «PantheCare», farmacia) |
| **Omnitheca** | La misma idea con el latín *omni* | Cogido | Libre | Libre | Ninguna |
| **Thesorium** | *Thesaurus* (tesoro, almacén) + «-orium»; en español evoca «tesoro» | Cogido | Libre | Libre | Ninguna |
| **Vitrinum** | La vitrina, latinizada | Cogido | Libre | Libre | Ninguna |
| **Keepsery** | *Keep* + «-ery», el sitio donde guardas | **Libre** | **Libre** | Libre | Ninguna |
| **Collectary** | *Collect* + «-ary» | Cogido | Libre | Libre | Ninguna |
| **Gatherium** | *Gather* + «-ium» | Cogido | Libre | Libre | Ninguna igual (solo «MEGATHERIUM», ropa) |

**Descartados:** Hoardary (.com y .app libres, pero *hoard* suena a acumulador compulsivo). **.app cogido:** omniteca, armarium, keepery, musarium, curatory, collectum, musaeum, scrinium, collectry, omnarium.

### Finalistas (26/09/2026)

Candidatos: **Pantheca**, **Thesorium** y **Gatherium**. Omnitheca se descarta: «todo» suena prepotente y falso, porque el catálogo está acotado. Musary y Colleo siguen como candidatos.

Validación: dominios por RDAP, Google Play y TMview con scripts propios. Se buscaron también variantes de escritura y pronunciación, porque una marca puede chocar por parecido.

| | Pantheca | Thesorium | Gatherium |
|---|---|---|---|
| **.app** | Libre | Libre | Libre |
| **.com** | Cogido desde 2016 (Joker.com), sin web | Cogido en junio de 2025 (hosting italiano), página por defecto | Cogido desde 2020: blog activo «Gather» sobre organizar eventos y encuentros |
| **Google Play** (nombre y variantes) | Libre (panteca, pantheka) | Libre (tesorium, thesaurium) | Libre (gatherum) |
| **Marcas iguales** | Ninguna | Ninguna | Ninguna |
| **Marcas parecidas** | PANTECA: panaderías y alimentación en Latinoamérica (29, 30, 35, 43); la de la 42, en Uruguay, está vencida | Ninguna (tesorium, thesaurium) | «Gatherum Collectif», registrada en Nueva Zelanda en la clase 9 |
| **Pega** | En español se pronuncia «panteca», que suena a **pan** (panadería), y así se llaman esas marcas | .com registrado hace poco: alguien puede tener planes | El .com es una web viva con un nombre casi igual |

**Dato para Gatherium:** *omnium gatherum* es una expresión inglesa que significa «colección de todo tipo de cosas». Encaja con el sentido, y explica que existan marcas parecidas.

### Variantes con k (26/09/2026)

Al maintainer le gusta la k. Validadas igual que los finalistas:

| | Pantheka | Panteka |
|---|---|---|
| **.com** | **Libre** | En venta como dominio «premium» (DomainMarket, registrado en 2015) |
| **.app** | **Libre** | Libre |
| **Google Play** | Libre | Libre |
| **Marcas** | Ninguna igual ni parecida | «Panteka», registrada en Francia (Swifteo SASU) en la **clase 28: juegos y juguetes**. No es una clase de conflicto, pero toca de cerca los juegos de mesa. Las demás son de alimentación y cosmética |

**Pantheka es el único candidato de todas las rondas con .com y .app libres, además de Google Play y marcas limpios.** Con la k se aleja un poco de las marcas de panadería «PANTECA», aunque en español se sigue pronunciando «panteka». En alemán, *-thek* es el sufijo de *Bibliothek* y *Spielothek*, así que se lee con naturalidad.

### Quinta ronda (26/09/2026): nombres más creativos

Con Pantheka como listón: imágenes y referencias de colección más allá de la etimología.

| Nombre | Idea | .com | .app | Google Play | Marcas en TMview |
|---|---|---|---|---|---|
| **Curioteka** | *Curio* + *-teka*: el gabinete de curiosidades | **Libre** | **Libre** | Libre | Ninguna |
| **Kosmotheka** | Griego *kosmos* (mundo ordenado, lo contrario del caos) + *theka*: el cosmos de tus cosas | **Libre** | **Libre** | Libre | Ninguna |
| Mytheka | «Mi *theka*», y evoca «mito» | Cogido | Libre | Libre | Ninguna |
| Keimelia | Griego *κειμήλια*: tesoros y recuerdos de familia que se guardan | Cogido | Libre | Libre | Ninguna. Difícil de pronunciar |
| Lendlore | *Lend* + *lore* | Cogido | Libre | Libre | Ninguna. Habla de prestar, no de coleccionar |
| Urraka | La urraca, que colecciona cosas brillantes | Cogido | Libre | Libre | **URRAKA registrada en Argentina, clase 41** |
| Kimelia | Variante de Keimelia | Cogido | Libre | Libre | **Kimélia registrada en Francia, clases 35 y 41** |
| Kuriosa | Curiosidades | Cogido | Libre | Libre | 60 marcas; **LaKuriosa en España, clase 41** |

**.app cogido:** picapica (nombre científico de la urraca), wunderly, mnemora, keepling, shelfling. Sin respuesta del registro: arkiva.

### Sexta ronda (26/09/2026): variaciones de *kosmos*

Al maintainer le gusta el enfoque de *kosmos*.

| Nombre | Idea | .com | .app | Google Play | Marcas en TMview |
|---|---|---|---|---|---|
| **Kosmodex** | *Kosmos* + *index*, como Pokédex o Rolodex: el índice de tu cosmos | Libre | Libre | Libre | Ninguna |
| **Kosmarium** | *Kosmos* + «-arium», como *planetarium* o *aquarium* | Libre | Libre | Libre | Ninguna igual (solo КОШМАРИУМ, «pesadilla», un juego de mesa ruso) |
| **Kosmothek** | Con el sufijo alemán *-thek* (*Bibliothek*) | Libre | Libre | Libre | Ninguna |
| **Kosmoteca** | Con el sufijo español e italiano «-teca» | Libre | Libre | Libre | Ninguna |
| **Kosmotheke** | Con el griego *theke* tal cual | Libre | Libre | Libre | Ninguna |
| **Kosmokeep** | *Kosmos* + *keep* | Libre | Libre | Libre | Ninguna |
| Kosmoteka | Error de escritura probable de Kosmotheka | Cogido | Libre | Libre | Ninguna |
| Kosmotek | Otro error probable | Cogido | Libre | Libre | Ninguna |
| Kosmora | — | Cogido | Libre | Libre | **«Kena: Scars of Kosmora», videojuego registrado en las clases 9 y 41**; KOSMORAMA en Noruega, 9 y 41 |
| Kosmion | — | Cogido | Libre | Libre | Joyería (Grecia) |

**Para Kosmotheka:** sus errores de escritura más probables (Kosmoteka, Kosmotek) están limpios en marcas y en Google Play, pero sus .com son de otros. Quien lo escriba mal por oído acabará en otra web.

**.app cogido:** mykosmos.

### Descartados

- **Marcas de software de bibliotecas:** Ex Libris y Aleph.
- **Demasiado usados:** Babel, Mimir y Trove.
- **Ya son apps de libros o colecciones** (de una lista de ChatGPT): Nook (Barnes & Noble), Trove, Curio, Kollect, Hoardy, Librio y Stash.
- **En uso en otros sectores:** Keevi, Keepio, Kivvi, Nolio, Tovio, Tavilo y Folko.
- **Dominio cogido:** anaquel.com, anaquel.app, tejuelo.com, lomario.com, estantia.com, lotengo.com, yalotengo.com, prestame.com, dejamelo.com, tomario.com, tomoteca.com, prestateca.com, ludoteka.com, balda.app, lomo.app, estante.app, lendshelf.com, shelfmark.com, spinewise.com, spineshelf.com, shelfhold.com, shelfkin.com, tomekeeper.com, tomewise.com, lendkin.com, ownshelf.com y havelist.com.

Varios de esos dominios se registraron en 2026: los nombres buenos vuelan. Cuando haya decisión, conviene registrarlo pronto.

## Siguientes pasos

1. Decidir el enfoque: nombre neutro o nombre que diga lo que hace.
2. Pasar por TMview los nombres de la primera ronda que sigan vivos (Funes, Kinshelf).
3. Con dos o tres favoritos:
   - Comprobar los .app que faltan y los .es.
   - Buscar por similitud en el buscador de la EUIPO (eSearch plus), no solo por texto literal.
   - Comprobar GitHub y Google Play.
   - Ver cómo aparecen en un buscador.
