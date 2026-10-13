# Documentación de Kosmotheka

> Esta carpeta está en español mientras construimos. El resto del repositorio (código, comentarios, commits, README raíz) va en inglés. Más adelante se traducirá.

## Índice

### Para trabajar

| Documento | Qué contiene |
|---|---|
| [ROADMAP.md](ROADMAP.md) | El plan: fases, qué se construye y cuándo está hecho |
| [DECISIONS.md](DECISIONS.md) | Registro de decisiones: contexto, opciones, decisión y estado |
| [plans/](plans/) | El plan de la fase en curso: [Fase 7](plans/fase-7.md). Y el [índice de la documentación pública](plans/documentacion-publica.md) |
| [architecture/](architecture/) | Cómo está construido: [modelo de datos](architecture/data-model.md), [operaciones](architecture/operations.md), [sincronización](architecture/sync.md) |
| [design/](design/) | [Brief de diseño](design/brief.md), [catálogo de errores](design/errors.md) y [pantallas de Claude Design](design/screens/README.md) |
| [development/](development/) | Entorno de desarrollo: [app](development/setup.md), [servidor](development/server.md) y [servicios externos que hay que configurar](development/servicios-externos.md) |
| [testing/](testing/) | Pruebas manuales: [hito 1](testing/hito-1.md) y [Fase 2](testing/fase-2.md) |
| [research/](research/) | Investigación: [motores de sincronización](research/sync-engines.md), [Koillection](research/koillection.md), [nombre del proyecto](research/naming.md), [apps de referencia](research/referentes.md) y [proveedores de datos](research/proveedores.md) |

### Histórico

No se actualizan: explican de dónde viene lo que hay ahora.

| Documento | Qué contiene |
|---|---|
| [VISION.md](VISION.md) | Visión original (ideación del 24/09/2026) |
| [historico/revision-alcance.md](historico/revision-alcance.md) | Debate del 25/09/2026 que replanteó el alcance: bibliotecas, personas y roles, nube, catálogos |
| [historico/plan-servidor-2026-09.md](historico/plan-servidor-2026-09.md) | Plan del servidor anterior a esa revisión |
| [historico/plan-fase-4.md](historico/plan-fase-4.md), [plan-fase-5.md](historico/plan-fase-5.md), [plan-fase-6.md](historico/plan-fase-6.md) | Planes de las fases ya cerradas |
| [historico/brief-diseno-fases-1-2.md](historico/brief-diseno-fases-1-2.md) | Brief de diseño de la app de las fases 1 y 2 |

## Convenciones de la documentación

- **Cada decisión tiene un estado**: `abierta` (se está discutiendo), `propuesta` (hay una recomendación pendiente de aceptar), `aceptada`, o `reemplazada por Dxx`. No se borran decisiones antiguas: se marcan como reemplazadas, así queda el porqué.
- **Trabajo e histórico, separados.** Un plan vive en `plans/` mientras se trabaja en él; cuando se termina o se sustituye, pasa a `historico/` y ya no se toca. Lo vigente está siempre en ROADMAP.md y DECISIONS.md.
- **Documentar a medida que se construye**, no al final. Cada fase del roadmap incluye como entregable la documentación que genera.
- Cuando llegue el momento de las guías (usuario, instalación, autoalojamiento, contribución), se organizarán siguiendo el esquema [Diátaxis](https://diataxis.fr/): tutoriales, guías prácticas, referencia y explicación. Lo que escribamos ahora en DECISIONS.md alimentará directamente la parte de «explicación».
