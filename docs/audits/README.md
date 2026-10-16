# Auditorías

Revisiones periódicas del proyecto: calidad, contratos entre capas, pruebas, costuras entre funcionalidades, integridad de los datos y seguridad. Las hace una sesión de Claude que solo audita. El procedimiento exacto, para Claude, está en `.claude/skills/audit/SKILL.md`; este documento lo explica para personas.

## Cómo se lanza

1. Comprueba que ninguna otra sesión está trabajando: todas gastan el mismo cupo.
2. En la sesión de auditoría, `/clear` para empezar con el contexto vacío.
3. `/audit`.

| Orden | Qué hace |
|---|---|
| `/audit` | Revisa lo que ha cambiado desde la última auditoría y un tema a fondo, por turnos. La primera vez hace la pasada inicial. |
| `/audit sync-convergence lan-pairing` | Revisa a fondo esos temas, dentro del presupuesto. |
| `/audit presupuesto alto` | Dobla el presupuesto de esa pasada. |

Cuándo: al cerrar cada hito del ROADMAP.

## Cómo funciona

La sesión coordinadora orquesta y nadie explora el repositorio por su cuenta. Hay cuatro capas, de la más barata a la más cara:

1. **Scripts, sin modelo.** `bin/prepare.sh` saca una copia fija del último commit de `main` con `git archive`, lanza los escáneres y ejecuta `bin/facts.py`. Todo corre en Docker con versiones fijadas y tarda unos 20 segundos. `facts.py` comprueba mecánicamente:
   - que coincidan el esquema de las migraciones, `schema.rs`, el JSON Schema y el espejo PHP;
   - los comandos de Tauri frente a las llamadas del front;
   - cada lista cerrada (formatos, tipos, roles, estados, géneros) y cada familia de códigos de error en Rust, TypeScript y las traducciones, según `bin/contracts.json`;
   - los campos de cada `struct` de Rust frente a la `interface` de TypeScript del mismo nombre;
   - las traducciones `en` frente a `es` (contando las claves que se construyen con un prefijo), los códigos de error y los casos de conformidad;
   - el código muerto de la app (`knip`): un comando que tiene función en `api.ts` pero ninguna pantalla que la use;
   - el inventario de tests y la superficie de ataque (CSP, capabilities, Android, Docker, `.env`);
   - y apunta los candidatos a problema: `unwrap`, SQL construido a mano, errores que se pierden, efectos de React…
2. **Hallazgos mecánicos.** Los escribe el coordinador directamente a partir de esos datos, sin agentes.
3. **Triaje.** Un agente ligero con esfuerzo bajo recibe un paquete con hasta 40 candidatos, cada uno con su trozo de código, y decide cuáles son de verdad un problema. Solo puede leer su paquete.
4. **Revisión por temas.** Un agente recibe el paquete de un tema, que ya trae las instrucciones, las preguntas, las decisiones y convenciones que aplican, los datos mecánicos y solo el código de las funciones implicadas. Puede hacer como mucho 8 consultas extra para confirmar una hipótesis.

Los paquetes los genera `bin/packet.py` a partir de `topics/<tema>.md`. En modo incremental solo llevan las funciones que han cambiado; si un tema no ha cambiado, no se lanza.

Los agentes no cargan `CLAUDE.md` ni `AGENTS.md`: su paquete lleva lo que necesitan. Tienen un tope de turnos y un nivel de esfuerzo fijo:

| Agente | Modelo | Esfuerzo | Para |
|---|---|---|---|
| `audit-triage` | ligero | bajo | Candidatos mecánicos |
| `audit-reviewer` | ligero | medio | La mayoría de los temas |
| `audit-reviewer-strong` | principal | medio | Modelo de amenazas, convergencia, permisos, emparejamiento, costuras de compartir |

El coordinador trabaja con esfuerzo medio. Para analizar un informe con calma puedes subirlo tú con `/effort`.

## Presupuesto por pasada

- Como mucho 2 revisiones, solo una con el modelo principal, y 3 paquetes de triaje.
- Con «presupuesto alto», el doble.
- Si quieres medir el consumo, pide lanzarla **por oleadas**: primero los scripts, luego los agentes ligeros y al final el del modelo principal, parando entre una y otra. La primera auditoría (03/10/2026) gastó unos 190.000 tokens en 7 agentes: unos 63.000 el modelo de amenazas con el modelo principal y entre 11.000 y 46.000 cada uno de los demás.
- La primera pasada revisa el modelo de amenazas y la cadena de suministro y tria los tres grupos de candidatos de más riesgo. El resto queda como «Pendiente» y va entrando en las pasadas siguientes.

## Temas y rotación

En cada pasada incremental se revisa un tema entero, por este orden de riesgo:

| # | Tema | Qué revisa |
|---|---|---|
| 1 | `sync-convergence` | Que los dispositivos converjan: HLC, borrados, lotes, versiones distintas de la app |
| 2 | `library-access` | Quién puede acabar con los datos de una biblioteca: red local, nube, id fijo, local-first y resincronizar por dos caminos |
| 3 | `permissions-on-receipt` | Que quien recibe compruebe los permisos, la retirada de acceso y las solicitudes |
| 4 | `lan-pairing` | Identidad, emparejamiento por QR y transporte en la LAN |
| 5 | `ipc-surface` | Comandos de Tauri, capabilities, CSP, claves y Android |
| 6 | `data-files` | SQLite, copias de seguridad, restaurar con sync, CSV y carpeta de datos |
| 7 | `migrations` | Migraciones que pueden perder datos o dejar la base a medias |
| 8 | `signatures` | Firmas en Rust y PHP: mismos bytes, diferencias entre los analizadores de JSON |
| 9 | `flows-sharing` | Costuras de compartir: roles sin conexión, solicitudes, retirada, personas |
| 10 | `flows-catalog` | Costuras del catálogo: mover libros, marcas, deseos, búsqueda |
| 11 | `webview-rendering` | Datos no fiables mostrados en la interfaz (XSS) |
| 12 | `server` | Servidor Symfony: seguridad y convenciones |
| 13 | `metadata-covers` | Proveedores externos y portadas sin conexión o con respuestas hostiles |
| 14 | `spec-coverage` | Que cada regla de la spec tenga casos de conformidad |
| 15 | `tests-quality` | QA: qué falta probar y si las pruebas detectarían un fallo |
| 16–20 | `ux-sharing` · `ux-catalog` · `ux-book` · `ux-shell` · `ux-wording` | Pantallas sin salida, errores, estados vacíos, textos |
| 21–22 | `quality-rust` · `quality-ts` | Calidad; en Rust explica el porqué de cada hallazgo, para aprender |
| 23 | `docs-vs-code` | Decisiones y hojas de ruta frente al código |
| 24 | `supply-chain` | Dependencias, secretos, licencias, CI y lo necesario para publicar |

`library-access` vuelve además, fuera de la rotación, cada vez que cambia quién recibe una biblioteca: al terminar los hitos 7.2, 7.5 y 7.6 (`/audit library-access`).

`threat-model` se escribe en la primera pasada y vuelve a revisarse solo cuando cambian decisiones de seguridad.

## Verificación

- Ningún hallazgo entra si la cita de código no está en el commit auditado.
- El coordinador comprueba en el código los críticos y altos.
- Lo descartado queda anotado con su motivo, para que no vuelva a salir.

## Qué se entrega

- `AAAA-MM-DD.md`: un informe por pasada, con lo nuevo, lo resuelto, lo que sigue abierto, lo que hay que decidir, escenarios de prueba manual y lo pendiente.
- `hallazgos.md`: el registro vivo. Cada hallazgo tiene un identificador fijo que no se reutiliza:
  - `SEG`: seguridad;
  - `DAT`: integridad de datos;
  - `COS`: costuras;
  - `CON`: contratos;
  - `PRU`: pruebas;
  - `CAL`: calidad.

  Basta con decir en la sesión de desarrollo «arregla SEG-999».
- `amenazas.md`: el modelo de amenazas.
- `descartes.md`: los candidatos mecánicos ya revisados que no son un problema. No vuelven a los paquetes de triaje mientras su fichero no cambie, así que cada pasada solo paga por lo nuevo.

La auditoría no arregla nada ni escribe fuera de esta carpeta. Tú decides qué se arregla y cuándo.

## Gravedad y confianza

| Gravedad | Cuándo |
|---|---|
| Crítica | En uso normal o para un atacante realista: pérdida o corrupción de datos que no se puede deshacer, dispositivos que ya no convergen, alguien lee o escribe lo que su rol o su dispositivo no permite, o un secreto real expuesto. |
| Alta | Se rompe una funcionalidad o una garantía en un caso realista, o hay un fallo de seguridad con una condición plausible: misma red, móvil robado, un miembro de mala fe, una app modificada. |
| Media | Casos raros, defensas que faltan, reglas importantes sin pruebas, deuda que dará fallos pronto. |
| Baja | Claridad, nombres, limpieza. |

La confianza puede ser **confirmada**, **probable** (depende de algo en ejecución) o **posible** (sospecha). Una sospecha nunca es crítica ni alta.

Estados de un hallazgo: `abierto`, `resuelto`, `aceptado (Dxx)` y `descartado`.

## Herramientas

Todas se ejecutan en Docker con la versión fijada. No se instala nada en la máquina.

| Herramienta | Versión | Qué hace | Qué sale de la máquina |
|---|---|---|---|
| osv-scanner | v2.6.0 | Dependencias vulnerables (Rust, npm, Composer) y licencias | Nombres y versiones de las dependencias, a osv.dev y deps.dev |
| gitleaks | v8.30.1 | Secretos en el historial de git | Nada |
| semgrep | 1.177.0 | Análisis estático de PHP, TypeScript, Rust, Docker, nginx y GitHub Actions | Nada; solo descarga las reglas |
| python | 3.13.15-slim | Ejecuta `facts.py` (SQLite moderno para aplicar las migraciones) | Nada |
| knip | 6.39.0 (en `node:22.20.0-slim`) | Exportaciones, ficheros y dependencias sin usar en la app | Nada; solo descarga knip. Usa el `node_modules` de tu copia, en solo lectura |

`packet.py` usa el `python3` de la máquina. Para cambiar una versión, se actualiza en `bin/prepare.sh` y aquí, y se indica en el informe siguiente.

## Condición para publicar el repositorio

Antes de hacerlo público:

- ningún hallazgo `SEG` crítico o alto abierto;
- gitleaks limpio en todo el historial, o los secretos rotados y el historial limpiado;
- un `SECURITY.md` que explique cómo avisar de una vulnerabilidad.

Los informes de seguridad se quedarán en el historial público. Para entonces, lo que describan tiene que estar resuelto.

## Lo que no cubre todavía

- **Lo que se ve en pantalla** (textos tapados, contraste en modo oscuro, botones fuera de la zona segura): irá en una skill aparte, `/audit-visual`, con capturas del modo `?mock` a ancho de móvil y su propio presupuesto, para lanzarla de vez en cuando (por ejemplo, antes de subir una versión a Play). Pendiente de construir.
- **Si los tests detectarían un fallo** (pruebas de mutación con `cargo-mutants`): primera pasada el 04/10/2026, antes de 7.4 ([informe](mutaciones-2026-10-04.md)). No va en el CI. Se lanza a mano, en Docker con una imagen fijada (`rust:1.98.1-bookworm` con las dependencias de Tauri del CI y `cargo-mutants` 27.1.0), sobre una copia limpia del repositorio (`git archive` más `app/dist`). Lo aprendido para la próxima:
  - Tardó 82 minutos con 319 mutantes y la máquina al límite: cada mutante de la app recompila y pasa sus 305 tests. Pasar solo los tests del módulo mutado (`-- sync::` o `-- permissions::`), lanzarlo solo sobre lo que cambió (`--in-diff`) y usar 2 o 3 trabajos en lugar de 4.
  - Los mutantes del dominio se prueban por defecto solo con los tests del dominio, y casi todo el dominio lo comprueba la app: o se lanza con `--test-workspace true`, o se repiten los «no detectados» del dominio así (en la primera pasada, 16 de 30 eran falsos).
