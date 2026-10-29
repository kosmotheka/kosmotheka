# Documentación pública: índice

Primer índice de lo que tendrá **docs.kosmotheka.com** (D90, `propuesta`), montado el 13/10/2026 para que al llegar a la Fase 8 no haya que empezar de cero. No hay ninguna página escrita: esto dice qué páginas habrá, de dónde sale cada una y cuáles no tienen todavía material.

> **Estado a 29/10/2026:** el índice está escrito y publicado en docs.kosmotheka.com, salvo **«Préstamos»**, que espera a que existan. Las tres partes tienen además páginas que no estaban en este índice: «Cómo buscar mejor», «La papelera», «Nombre y logos» del servidor, «Método de auditoría». Lo que va en una sola página en lugar de dos: «¿Necesito un servidor?» y «Requisitos» (`run-your-server/index.md`); el correo, Google y las claves de datos (`configuration.md`); «Por qué así y no de otra forma» (al final de `sync-engine.md`); «Añadir un campo o una entidad» (`how-to-contribute.md`); el registro de decisiones y la hoja de ruta (enlazados desde `contribute/index.md`, en español). Las tablas de abajo se quedan como estaban, para ver de dónde salió cada página.

Lo que ya está decidido o propuesto en D90 y no se repite aquí: Markdown en el repo, convertido con un generador estático (Starlight propuesto), en español e inglés, alojado por nosotros, y organizado según [Diátaxis](https://diataxis.fr/).

## Cómo leer las tablas

- **Tipo** (Diátaxis): *tutorial* (aprender haciendo, de principio a fin), *guía* (resolver una tarea concreta), *referencia* (datos para consultar), *explicación* (por qué es así).
- **Material:** de dónde sale. «Nada» significa que hay que escribirla desde cero.
- **Cuándo:** **A** antes de la prueba cerrada de Google Play; **B** antes de abrir el repositorio; **C** puede esperar.

## 1 · Quien usa la app

Sin jerga. Cada página responde a una pregunta que alguien se hace con la app en la mano.

| Página | Tipo | Material | Cuándo |
|---|---|---|---|
| Qué es Kosmotheka y qué no es | explicación | Landing (`web/`), `VISION.md` | A |
| Primeros pasos: instalar y añadir tu primer libro | tutorial | Bienvenida de la app (D91), `testing/hito-1.md` | A |
| Añadir escaneando, por ISBN o buscando por título | guía | `testing/ux-escanear-y-editar.md`, Ayuda de la app («search») | A |
| Libros, juegos, cómics, películas, música: qué cambia en cada tipo | guía | `testing/fase-6.md` | A |
| Ejemplares, ubicaciones y quién es el dueño | guía | Glosario de `AGENTS.md` | A |
| Leído, valoración, favorito, «me falta» y «lo deseo» | guía | `testing/fase-6.md` | A |
| Series y huecos | guía | `testing/fase-6.md` | C |
| Préstamos | guía | Nada | A |
| Compartir en casa: emparejar dispositivos por wifi | tutorial | Ayuda de la app («pairing», «share»), `testing/fase-2.md` | A |
| Personas y permisos: administrador, editor, lector, peticiones | guía | `testing/fase-5.md` | A |
| Varias bibliotecas | guía | `testing/fase-4.md` | C |
| La nube: crear cuenta, subir una biblioteca, invitar | tutorial | `testing/fase-7.md` | A |
| Fuera de casa y sin conexión: qué funciona | explicación | Ayuda de la app («away») | A |
| Copias de seguridad, exportar e importar | guía | Ayuda de la app («backups») | A |
| He perdido el móvil o lo he cambiado | guía | Ayuda de la app («lost») | A |
| Quitar a una persona o un dispositivo | guía | `testing/fase-7.md` | A |
| Borrar mi cuenta y mis datos | guía | Nada | A (Google Play lo exige) |
| De dónde salen los datos: fuentes, claves propias y cómo apagarlas | referencia | `research/proveedores.md`, pantalla de Proveedores | A |
| Qué sale de tu dispositivo y qué no | explicación | `publicacion/politica-privacidad.md`, D91 | A |
| Mensajes de error y qué hacer | referencia | `design/errors.md` | C |
| Preguntas frecuentes | referencia | Página de preguntas de la landing | A |
| Novedades de cada versión | referencia | `publicacion/novedades.md`; después, el fichero de novedades de la app | A |

## 2 · Quien monta su propio servidor

Alguien que sabe administrar un servidor, pero no conoce el proyecto.

| Página | Tipo | Material | Cuándo |
|---|---|---|---|
| ¿Necesito un servidor? Qué aporta y qué no | explicación | `ROADMAP.md` (Fase 7), `architecture/cloud.md` | B |
| Requisitos | referencia | `site/…/run-your-server/index.md` (escrita) | B |
| Instalar con Docker | tutorial | `development/server.md`, `server/docker/` | B |
| Instalar sin Docker (nginx, PHP-FPM, PostgreSQL) | tutorial | `site/…/run-your-server/without-docker.md` (escrita) | B |
| Configuración: todas las variables | referencia | `server/.env`, que ya las comenta una a una | B |
| Correo saliente | guía | `development/correo.md` | B |
| Entrar con Google (opcional) | guía | `development/google-oauth.md` | B |
| Claves de las fuentes de datos (opcional) y sus condiciones | guía | `research/proveedores.md`, `development/servicios-externos.md` | B |
| Tareas programadas y limpieza | guía | `site/…/run-your-server/without-docker.md` y `administration.md` (escritas) | B |
| Copias de seguridad y restauración | guía | Nada | B |
| Actualizar a una versión nueva | guía | `site/…/run-your-server/updating.md` (escrita) | B |
| Apuntar la app a tu servidor | guía | Nada | B |
| Qué guarda el servidor y qué puede ver quien lo administra | explicación | `architecture/cloud.md`, `architecture/security.md` | B |
| Registros: qué se escribe y qué no | referencia | `site/…/run-your-server/what-the-server-keeps.md` (escrita) | B |
| Problemas frecuentes | referencia | Nada escrito; lo aprendido en el primer despliegue | C |

## 3 · Quien quiere colaborar o entender cómo funciona

| Página | Tipo | Material | Cuándo |
|---|---|---|---|
| Visión general: las piezas y cómo encajan | explicación | `AGENTS.md`, `README.md` | B |
| Preparar el entorno: app | tutorial | `development/setup.md` | B |
| Preparar el entorno: servidor | tutorial | `development/server.md` | B |
| Cómo contribuir: estilo, pruebas, commits | guía | `AGENTS.md`, `server/AGENTS.md`; falta `CONTRIBUTING.md` | B |
| Añadir un campo o una entidad | guía | `AGENTS.md` (la regla de los cuatro sitios) | B |
| Añadir una fuente de datos | guía | Nada; el patrón de `metadata/` | C |
| Añadir un idioma | guía | Nada | C |
| Modelo de datos | referencia | `architecture/data-model.md` | B |
| Operaciones: la especificación y los casos de conformidad | referencia | `architecture/operations.md`, `spec/` | B |
| El motor de sincronización, explicado | explicación | `architecture/sync.md`, `research/sync-engines.md` | B |
| Por qué así y no de otra forma: comparación con otros enfoques | explicación | `research/sync-engines.md`, `DECISIONS.md` | B |
| Cuentas e inicio de sesión | explicación | `architecture/accounts.md` | B |
| La nube: API y reglas de acceso | referencia | `architecture/cloud.md` | B |
| Seguridad: modelo de amenazas | explicación | `architecture/security.md`, `audits/amenazas.md` (se publica, D90) | B |
| Cómo auditamos, para repetirlo por tu cuenta | guía | `audits/README.md` (el método se publica; los resultados, no) | B |
| Avisar de una vulnerabilidad | guía | `SECURITY.md` | B |
| Registro de decisiones | referencia | `DECISIONS.md` | B |
| Glosario | referencia | `AGENTS.md` | B |
| Publicar una versión | guía | `site/…/contribute/release.md` (escrita), D74 | C |
| Hoja de ruta | referencia | `ROADMAP.md` | B |

## Lo que sale de este primer índice

- **La parte de usuario es la que menos material tiene.** Lo que hay son listas de pruebas manuales y la Ayuda de la app, no guías. Es además la que hace falta antes: 19 de sus 22 páginas van marcadas con A.
- **Tres páginas no tienen nada y son necesarias pronto:** préstamos, borrar la cuenta y los datos (Google Play lo exige en cuanto hay cuentas) y, para quien aloja, copias de seguridad y restauración del servidor.
- **La parte de colaboradores ya está casi escrita**, pero en español y como documentos de trabajo: hay que reordenarla y traducirla (D02), no escribirla.
- **La Ayuda de la app y la documentación deberían decir lo mismo.** Por decidir: si la Ayuda enlaza a la web o si los dos textos salen de un mismo sitio.

## Hablado con el maintainer el 13/10/2026

- **Acordado:** subdominio docs.kosmotheka.com, enlazado desde el pie de la web, y en los dos idiomas.
- **Todavía no se escribe ninguna página.** Las funciones siguen cambiando, y lo que hay hoy vive en los planes y en las listas de pruebas. Este índice se mantiene; las páginas se escriben cuando la fase esté estable.
- **Generador: Starlight** (aceptado por el maintainer el 13/10/2026, tras dudar entre eso y HTML hecho por nosotros). Por qué, frente a un script propio que convierta el Markdown:
  - Trae hecho lo que habría que construir y mantener: menú, dos idiomas con sus enlaces cruzados y buscador sin servidor.
  - Un script propio sería una pieza más nuestra, justo lo que preocupa.
  - Solo se usa al construir: lo que se publica es HTML estático, sin nada suyo en marcha en el servidor.
  - El aspecto se ajusta con los estilos de la landing, para que no parezca otro sitio.
  - La fuente sigue siendo Markdown en el repo. Si un día se cambia de generador, las páginas no se reescriben.
  - No se instala nada todavía: se monta cuando haya páginas que publicar.
- **Preocupación del maintainer: ya son varias piezas sueltas** además de la app y el servidor. Hoy son tres sitios estáticos: la landing (`web/`, kosmotheka.com), la página de invitaciones (`docs/publicacion/kosmotheka.app/`, kosmotheka.app) y esta documentación. Para que no se vaya de las manos, cuando se monte:
  - unos solos estilos para los tres (la landing y la página de invitaciones son HTML a mano; la documentación los toma prestados);
  - una sola forma de desplegarlos (D90 ya propone que se publiquen solos al cambiar `main`);
  - y una sola página que diga qué sitios hay, dónde está su fuente y cómo se publican ([web.md](../publicacion/web.md)).

## Por decidir, cuando toque

1. El despliegue. El subdominio, los dos idiomas y el generador ya están acordados.
2. En qué idioma se escribe primero la parte de usuario. Propuesta: español, que es en el que se prueba, y se traduce al cerrar cada página.
3. Si las capturas de pantalla se hacen a mano o se generan.
4. Si la Ayuda de la app y la web comparten texto.
