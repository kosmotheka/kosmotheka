---
title: Cómo auditamos, para repetirlo por tu cuenta
description: "El método de auditoría del proyecto, en dos partes: scripts que dan hechos y revisiones de juicio sobre paquetes de código preparados."
sidebar:
  order: 9
---

Cada cierto tiempo el proyecto se audita: calidad, contratos entre capas, pruebas, integridad de los datos y seguridad. Esta página explica **el método**, para que puedas repetirlo en tu copia, con la herramienta que prefieras: una persona, o cualquier asistente de programación, no solo el que usamos nosotros.

**Se publican el método y los hallazgos ya cerrados; los abiertos, cuando se cierran.** Así nadie lee un fallo antes de que esté arreglado, y los identificadores que cita el código (`SEG-…`) llevan a algún sitio. El procedimiento, además, sirve a quien quiera revisar su propia copia o un cambio que prepara.

## La idea

Una auditoría mezcla trabajo mecánico y trabajo de juicio, y conviene no mezclarlos:

1. **Lo mecánico lo hacen scripts, sin modelo ni juicio.** Sacan una copia fija del código, pasan escáneres y calculan hechos comprobables («¿coinciden estas dos listas?»). Cuestan casi nada y siempre dan lo mismo.
2. **Lo que pide juicio lo hace quien audita** (una persona o un modelo), **sobre un paquete de código ya preparado**: las preguntas, las decisiones que aplican y solo el código que importa. Nadie explora el repositorio entero.
3. **Todo lo que se afirma se verifica** contra el código antes de entrar en un informe.

*Por qué así:* explorar un repositorio entero es lo más caro y lo menos fiable. Con un paquete cerrado, el trabajo es reproducible, cuesta poco y se puede repartir.

## Qué necesitas

- Una copia del repositorio con Git.
- **Docker.** Todas las herramientas se ejecutan en contenedores con versión fijada: no se instala nada en tu máquina.
- `python3` en tu máquina (solo biblioteca estándar) para preparar los paquetes.
- Alguien (o algo) que haga las revisiones: una persona, o el modelo que quieras.

Los scripts y las reglas están en [`.claude/skills/audit/`](https://github.com/kosmotheka/kosmotheka/tree/main/.claude/skills/audit): `bin/` con los scripts, `topics/` con los temas, y `review-rules.md` y `triage-rules.md` con las reglas de las revisiones. La carpeta se llama así porque el maintainer la lanza con una orden de Claude Code (`/audit`), pero **nada del método depende de eso**: es texto, scripts y Docker.

## Parte 1: la parte mecánica

El script `bin/prepare.sh` hace, en unos 20 segundos:

1. Saca con `git archive` una **copia fija** del último commit de `main`, para auditar siempre algo concreto. Si le das el commit de la auditoría anterior, también calcula qué ha cambiado desde entonces.
2. Lanza los escáneres, **todos en Docker con versión fijada**:

| Herramienta | Versión | Qué hace | Qué sale de la máquina |
|---|---|---|---|
| osv-scanner | v2.6.0 | Dependencias con fallos conocidos (Rust, npm, Composer) y licencias | Nombres y versiones de las dependencias, a osv.dev y deps.dev |
| gitleaks | v8.30.1 | Secretos en el historial de Git | Nada |
| semgrep | 1.177.0 | Análisis estático de PHP, TypeScript, Rust, Docker, nginx y GitHub Actions | Nada; solo descarga las reglas |
| knip | 6.39.0 | Exportaciones, archivos y dependencias sin usar en la app | Nada; solo descarga knip |
| python | 3.13.15 | Ejecuta `facts.py` | Nada |

3. Ejecuta `bin/facts.py`, que escribe **hechos comprobables** en una carpeta, con un resumen (`SUMMARY.md`) que marca lo que merece atención:
   - que el esquema de las migraciones, `schema.rs`, el JSON Schema y el espejo en PHP **coincidan**;
   - que cada comando de Tauri que llama la interfaz exista, y con los mismos argumentos;
   - cada **lista cerrada** (formatos, tipos, roles, estados, géneros) y cada familia de códigos de error, comparadas entre Rust, TypeScript y las traducciones, según `bin/contracts.json`;
   - los campos de cada `struct` de Rust frente a la `interface` de TypeScript del mismo nombre;
   - las traducciones en inglés frente a las españolas, y los casos de conformidad frente a quien los ejecuta;
   - el inventario de pruebas y la superficie de ataque (CSP, permisos, Android, Docker, variables de entorno);
   - los **candidatos a problema**: `unwrap`, SQL construido a mano, errores que se pierden, efectos de React…

### Cómo se lee

- Se abre solo `SUMMARY.md`, y de ahí **solo las secciones que marca** (un aviso, un número distinto de cero).
- Una diferencia mecánica entre dos listas **no se descarta como «error del script» sin comparar a mano las dos fuentes reales.**
- Los hechos se convierten en hallazgos directamente: una lista a la que le falta un valor en TypeScript es un problema de contratos de gravedad media; un campo que la interfaz lee y Rust ya no manda es de gravedad alta.
- Los candidatos a problema **no son hallazgos todavía**: pasan por el triaje de la parte 2.

## Parte 2: la parte de juicio

### Triaje: ¿esto es un problema de verdad?

Los candidatos mecánicos son muchos y casi todos son falsos positivos (un `unwrap` sobre una constante, por ejemplo). El triaje decide cuáles son reales. Cada **paquete de triaje** lleva hasta 40 candidatos, cada uno con su trozo de código (la función entera si es corta), y **las reglas de su categoría**. Por ejemplo:

- Un `unwrap` sobre datos que vienen de otro dispositivo, del servidor o de la red es un problema serio (un par defectuoso podría hacer caer la app); sobre un archivo local o una entrada del usuario, uno menor; sobre una constante, no es nada.
- SQL construido con `format!`: es un problema si se interpola un **valor**; no lo es si solo se interpolan nombres de tabla o de columna que vienen de constantes.
- Una escritura en una tabla del catálogo fuera de donde debe estar (`ops.rs` y `sync.rs`) es siempre grave, porque se salta la regla de que toda escritura es una operación.

Quien hace el triaje puede leer **solo su paquete**. Responde con una tabla: qué candidatos son hallazgos y por qué, cuáles no, y cuáles «no concluyentes» (los que el trozo no basta para decidir).

### Revisión por temas

Cada **tema** es una pregunta amplia con su propio paquete. Los temas, por orden de riesgo:

| # | Tema | Qué revisa |
|---|---|---|
| 1 | `sync-convergence` | Que los dispositivos converjan: reloj, borrados, lotes, versiones distintas de la app |
| 2 | `library-access` | Quién puede acabar con los datos de una biblioteca |
| 3 | `permissions-on-receipt` | Que quien recibe compruebe los permisos y la retirada de acceso |
| 4 | `lan-pairing` | Identidad, emparejamiento por QR y transporte en la red local |
| 5 | `ipc-surface` | Comandos de Tauri, permisos, CSP, claves y Android |
| 6 | `data-files` | SQLite, copias de seguridad, restaurar con sincronización, CSV |
| 7 | `migrations` | Migraciones que pueden perder datos o dejar la base a medias |
| 8 | `signatures` | Firmas en Rust y PHP: los mismos bytes, diferencias entre analizadores de JSON |
| 9–10 | `flows-sharing`, `flows-catalog` | Costuras entre funciones: roles sin conexión, peticiones, mover libros, marcas |
| 11 | `webview-rendering` | Datos no fiables mostrados en la interfaz |
| 12 | `server` | El servidor Symfony: seguridad y convenciones |
| 13 | `metadata-covers` | Servicios de datos y portadas, sin conexión o con respuestas hostiles |
| 14–15 | `spec-coverage`, `tests-quality` | Que cada regla tenga casos, y si las pruebas detectarían un fallo |
| 16–20 | `ux-*` | Pantallas sin salida, errores, estados vacíos, textos |
| 21–22 | `quality-rust`, `quality-ts` | Calidad |
| 23 | `docs-vs-code` | Decisiones y hojas de ruta frente al código |
| 24 | `supply-chain` | Dependencias, secretos, licencias, integración continua |

Se revisa un tema entero por pasada, rotando, y el tema vuelve antes si su código ha cambiado. El modelo de amenazas (`threat-model`) se escribe una vez y se revisa solo cuando cambian decisiones de seguridad.

Cada tema es un archivo de `topics/` con una cabecera (qué código incluir, qué documentos de contexto, qué decisiones aplican, el tamaño máximo) y una lista de **preguntas concretas**. El script `bin/packet.py` convierte ese archivo en un **paquete**: las instrucciones, las preguntas, las decisiones, los hechos y solo el código de las funciones implicadas, con números de línea. Por ejemplo, las preguntas de `sync-convergence` piden razonar con líneas de tiempo concretas: «los dispositivos A, B y C, las operaciones que hace cada uno, el orden en que llegan y el estado final de cada uno. Una línea de tiempo que acaba en estados distintos, o un cambio perdido sin avisar, es un hallazgo».

### Las reglas de quien revisa

Se resumen en `review-rules.md`. Las más importantes:

- **Trabajar con el paquete.** Contestar a cada pregunta desde el código que trae, sin explorar el repositorio. Como mucho ocho comprobaciones extra, solo para confirmar o descartar una hipótesis concreta, y siempre de solo lectura.
- **Las decisiones aceptadas son la intención del proyecto.** Lo que una decisión elige no es un hallazgo, salvo que el código la contradiga o haya un riesgo que la decisión no menciona.
- **Sin cita, no hay hallazgo.** Cada hallazgo cita el código exacto con su ruta y sus líneas. «Falta X» exige prueba: decir qué código se comprobó.
- **Una causa, un hallazgo**, y como mucho doce por tema.
- **Un escenario concreto:** quién hace qué, en qué estado, y cuál es el resultado equivocado.

### Gravedad y confianza

| Gravedad | Cuándo |
|---|---|
| Crítica | En uso normal o para un atacante realista: pérdida o corrupción de datos que no se puede deshacer, dispositivos que ya no convergen, alguien lee o escribe lo que su rol no permite, o un secreto real expuesto |
| Alta | Se rompe una funcionalidad o una garantía en un caso realista, o hay un fallo de seguridad con una condición plausible: misma red, móvil robado, un miembro de mala fe, una app modificada |
| Media | Casos raros, defensas que faltan, reglas importantes sin pruebas, deuda que dará fallos pronto |
| Baja | Claridad, nombres, limpieza |

Ante la duda entre dos niveles, **el más bajo**, diciendo qué lo subiría. La confianza puede ser **confirmada** (se trazó el camino en el código), **probable** (un paso depende de algo en ejecución) o **posible** (una sospecha, que nunca es crítica ni alta).

### Verificación

Una auditoría llena de falsos positivos es peor que ninguna. Por eso:

- **Ningún hallazgo entra si su cita no está en el código auditado.**
- Los críticos y altos se comprueban a mano: se busca la cita y se leen unas líneas alrededor. Se confirma el escenario o se baja o descarta con el motivo. Si depende de algo en ejecución, queda como «probable» con pasos concretos para comprobarlo en pantalla.
- Se buscan duplicados entre revisiones y contra los hallazgos ya registrados.
- Lo que una decisión aceptada elige pasa a «aceptado» con su número; lo que necesita una decisión nueva va a una lista «Para decidir». **La auditoría nunca escribe en el registro de decisiones.**
- Lo descartado se anota con su motivo, para que no vuelva a salir.

### Qué se registra

Cada hallazgo tiene un identificador fijo que no se reutiliza, con un prefijo según su familia: seguridad, integridad de datos, costuras entre funciones, contratos entre capas, pruebas o calidad. Los estados son `abierto`, `resuelto`, `aceptado` (con la decisión) y `descartado`. Cada pasada escribe un informe con lo nuevo, lo resuelto, lo que sigue abierto, lo que hay que decidir, escenarios de prueba manual y lo pendiente. La auditoría **no arregla nada**: quien mantiene el proyecto decide qué se arregla y cuándo.

## Repetirlo por tu cuenta

1. **Prepara.** Desde la raíz del repositorio, ejecuta `.claude/skills/audit/bin/prepare.sh <repo> <carpeta-de-trabajo> [commit-anterior]`. Tarda unos veinte segundos.
2. **Lee los hechos.** Abre solo `facts/SUMMARY.md` y las secciones que marca. Convierte en hallazgos los hechos mecánicos, comprobando a mano las dos fuentes.
3. **Elige el trabajo.** Un tema de la tabla (el siguiente por riesgo, o el que toca tu cambio) y, si quieres, un grupo de candidatos para el triaje.
4. **Construye el paquete.** `python3 .claude/skills/audit/bin/packet.py review <carpeta> <tema>` (o `triage <carpeta> "<categorías>"`). Imprime la ruta y el tamaño. Con `--incremental` solo incluye lo que cambió desde la auditoría anterior.
5. **Dáselo a quien revise.** El paquete es un único archivo de texto con todo lo que hace falta: puedes leerlo tú, o dárselo a cualquier asistente con la instrucción de seguirlo tal cual y contestar en el formato que pide. No necesita acceso al repositorio.
6. **Verifica.** Sigue las reglas de verificación de arriba para cada respuesta antes de aceptarla.
7. **Escribe y limpia.** Anota lo encontrado donde prefieras, y borra la carpeta de trabajo (es una copia del código).

Un consejo: **audita poco y a menudo**. Dos temas bien revisados por pasada valen más que veinte a medias, y por eso el método tiene un presupuesto: como mucho dos revisiones por temas por pasada y tres paquetes de triaje.

## Lo que todavía no cubre

- **Lo que se ve en pantalla** (textos tapados, contraste en modo oscuro, botones fuera de la zona segura): hace falta una revisión visual con capturas, que aún no está construida.
- **Si las pruebas detectarían un fallo** (pruebas de mutación, que cambian el código a propósito para ver si alguna prueba se queja): se hicieron a mano una vez, no forman parte del método ni de la integración continua.
- **Pruebas en ejecución:** el método lee código y escáneres; no lanza ataques contra un servidor en marcha ni fuzzing. Lo que depende de algo en ejecución queda como «probable» y con pasos para probarlo a mano.
- **Una auditoría externa.** Esto es una revisión sistemática hecha por quien mantiene el proyecto y por modelos, no una auditoría independiente ni una prueba de penetración. Un modelo puede equivocarse: por eso todo se verifica contra el código.
