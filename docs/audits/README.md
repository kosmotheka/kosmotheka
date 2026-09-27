# Auditorías

Revisiones periódicas del proyecto: calidad del código, contratos entre capas, pruebas, costuras entre funcionalidades, integridad de los datos y seguridad. Las hace una sesión de Claude dedicada solo a auditar, en paralelo con la de desarrollo. El procedimiento exacto, para Claude, está en `.claude/skills/audit/SKILL.md`; este documento lo explica para personas.

## Cómo se lanza

| Orden | Qué hace |
|---|---|
| `/audit` | Revisa lo que ha cambiado desde la última auditoría. Si no hay ninguna, hace la inicial. |
| `/audit initial` | Revisa el repositorio entero. |
| `/audit audit-flows audit-tests` | Revisa a fondo solo esos auditores, sobre todo su ámbito. |

Cuándo: al cerrar cada hito del ROADMAP. Una auditoría incremental cuesta en proporción a lo que haya cambiado.

## Los auditores

Son agentes de solo lectura. Leen una copia fija del commit auditado, sacada con `git archive`: lo que la sesión de desarrollo tenga a medias no entra y nadie toca el árbol de trabajo. No repiten lo que ya hace CI (formato, clippy, tests, ESLint, TypeScript, PHPStan, Rector, PHPUnit).

| Auditor | Qué revisa | Modelo |
|---|---|---|
| `audit-quality-rust` | Convenciones de AGENTS.md en Rust, pánicos, errores que se pierden, transacciones, código muerto. Explica el porqué de cada hallazgo, para aprender Rust. | ligero |
| `audit-quality-ts` | Lógica de dominio en React, textos sin traducir, errores de los comandos, efectos y carreras, accesibilidad básica. | ligero |
| `audit-quality-php` | Las convenciones de `server/AGENTS.md`, tipos, errores, código muerto. | ligero |
| `audit-contracts` | Que coincidan las piezas que deben decir lo mismo: migraciones, `schema.rs`, JSON Schema, `CatalogEntity.php`, comandos Tauri y front, traducciones `en`/`es`, códigos de error, matriz de permisos, firmas en Rust y PHP. | ligero |
| `audit-tests` | QA: si lo que importa está probado y si esas pruebas detectarían un fallo. Propone las pruebas que faltan. | ligero |
| `audit-supply-chain` | Dependencias vulnerables, secretos en todo el historial, CI, Docker, licencias y lo necesario para publicar el repositorio. | ligero |
| `audit-flows` | Costuras entre funcionalidades y de uso: flujos que cruzan varias piezas, callejones sin salida, errores mal mostrados, documentación que no coincide con el código. Propone escenarios de prueba manual. | principal |
| `audit-data-integrity` | Que no se pierdan ni se corrompan datos y que los dispositivos converjan: HLC, lápidas, transacciones, migraciones, versiones distintas de la app, copias de seguridad, CSV. | principal |
| `audit-security-design` | Modelo de amenazas y seguridad del diseño: identidad, emparejamiento, firmas, roles, retirada de acceso y privacidad. | principal |
| `audit-security-server` | El servidor Symfony, su configuración y Docker, siguiendo OWASP API Security Top 10 y ASVS. | principal |
| `audit-security-client` | La app Tauri en escritorio y Android: comandos IPC, capabilities, CSP, XSS, servicio LAN, claves, peticiones salientes, siguiendo el modelo de seguridad de Tauri y OWASP MASVS. | principal |

Los auditores del modelo ligero siguen listas de comprobación cerradas, con las búsquedas exactas que tienen que hacer. Los del modelo principal razonan sobre escenarios.

En cada auditoría incremental, además, un auditor revisa entero su ámbito, por turnos, para que el código antiguo no se quede sin mirar.

## Verificación

Los auditores proponen y la sesión coordinadora verifica. Ningún hallazgo entra en el informe si la cita de código no está en el commit auditado. Los de gravedad crítica o alta se comprueban leyendo el camino completo en el código. Lo que se descarta queda anotado con su motivo, para que no vuelva a salir.

## Qué se entrega

- `AAAA-MM-DD.md`: un informe por auditoría, con el commit revisado, un resumen, lo nuevo, lo resuelto, lo que sigue abierto, lo que hay que decidir y escenarios de prueba manual.
- `hallazgos.md`: el registro vivo. Cada hallazgo tiene un identificador fijo que no se reutiliza: `SEG` (seguridad), `DAT` (integridad de datos), `COS` (costuras), `CON` (contratos), `PRU` (pruebas), `CAL` (calidad). Basta con decir en la sesión de desarrollo «arregla SEG-003».
- `amenazas.md`: el modelo de amenazas. Recoge activos, atacantes, fronteras de confianza, amenazas por frontera y riesgos aceptados.

La auditoría no arregla nada ni escribe fuera de esta carpeta. Tú decides qué se arregla y cuándo. Si un hallazgo pide una decisión de arquitectura, aparece en «Para decidir».

## Gravedad y confianza

| Gravedad | Cuándo |
|---|---|
| Crítica | En uso normal, o para un atacante del modelo de amenazas: pérdida o corrupción de datos que no se puede deshacer, dispositivos que ya no convergen, alguien lee o escribe lo que su rol o su dispositivo no permite, o un secreto real en el repositorio o su historial. |
| Alta | Se rompe una funcionalidad o una garantía en un caso realista, o hay un fallo de seguridad con una condición plausible: misma red, móvil robado desbloqueado, un miembro de mala fe. |
| Media | Casos raros, defensas que faltan, reglas importantes sin pruebas, deuda que dará fallos pronto. |
| Baja | Claridad, nombres, limpieza. |

La confianza puede ser **confirmada** (se ha seguido el camino en el código), **probable** (depende de algo en ejecución) o **posible** (sospecha). Una sospecha nunca es crítica ni alta.

Estados de un hallazgo: `abierto`, `resuelto`, `aceptado (Dxx)` si una decisión asume el riesgo, y `descartado`.

## Herramientas

Todas se ejecutan en Docker con la versión fijada. No se instala nada en la máquina.

| Herramienta | Versión | Qué hace | Qué sale de la máquina |
|---|---|---|---|
| osv-scanner | v2.6.0 | Dependencias vulnerables (Rust, npm, Composer) y licencias | Nombres y versiones de las dependencias, a osv.dev y deps.dev |
| gitleaks | v8.30.1 | Secretos en todo el historial de git | Nada |
| semgrep | 1.177.0 | Análisis estático de PHP, TypeScript, Rust, Docker, nginx y GitHub Actions | Nada; solo descarga las reglas |

Para cambiar de versión, se actualiza aquí y en la skill, y se indica en el informe siguiente.

## Condición para publicar el repositorio

Antes de hacerlo público:

- ningún hallazgo `SEG` crítico o alto abierto;
- gitleaks limpio en todo el historial, o los secretos rotados y el historial limpiado;
- un `SECURITY.md` que explique cómo avisar de una vulnerabilidad.

Los informes de seguridad se quedarán en el historial público. Para entonces, lo que describan tiene que estar resuelto.
