# Documentación del proyecto

> Esta carpeta está en español mientras construimos. El resto del repositorio (código, comentarios, commits, README raíz) va en inglés. Más adelante se traducirá.

## Índice

| Documento | Qué contiene | Estado |
|---|---|---|
| [VISION.md](VISION.md) | Visión original del proyecto (ideación del 24/09/2026). Documento histórico: las decisiones actualizadas están en DECISIONS.md. | Cerrado |
| [DECISIONS.md](DECISIONS.md) | Registro de decisiones de arquitectura: contexto, opciones, recomendación y estado de cada una. | Vivo |
| [ROADMAP.md](ROADMAP.md) | Plan de ejecución por fases. | Borrador |
| [research/](research/) | Notas de investigación: [motores de sincronización](research/sync-engines.md), [Koillection](research/koillection.md) y [nombre del proyecto](research/naming.md). | Vivo |
| [development/setup.md](development/setup.md) | Cómo montar el entorno de desarrollo. | Vivo |
| [development/server.md](development/server.md) | Servidor Symfony en Docker: arranque, puertos, bases de datos y comandos. | Vivo |
| [plans/fase-3.md](plans/fase-3.md) | Plan detallado de la Fase 3 (servidor) y sus decisiones (D41–D53; D52 y D53 abiertas). | Vivo |
| [architecture/data-model.md](architecture/data-model.md) | Esquema de la base de datos local. | Vivo |
| [architecture/operations.md](architecture/operations.md) | Cómo se escribe: operaciones, HLC y validación. | Vivo |
| [testing/hito-1.md](testing/hito-1.md) | Pruebas manuales del hito 1. | Vivo |
| [testing/fase-2.md](testing/fase-2.md) | Pruebas manuales de la Fase 2. | Vivo |
| [design/brief.md](design/brief.md) | Brief de diseño de interfaz (para Claude Design u otro diseñador). | Vivo |
| [design/errors.md](design/errors.md) | Catálogo de errores y avisos: mensaje, causa y qué hacer; guía «El móvil no se empareja». | Vivo |
| [architecture/sync.md](architecture/sync.md) | Sincronización entre dispositivos: reglas y protocolo. | Vivo |

## Convenciones de la documentación

- **Cada decisión tiene un estado**: `abierta` (se está discutiendo), `propuesta` (hay una recomendación pendiente de aceptar), `aceptada`, o `reemplazada por Dxx`. No se borran decisiones antiguas: se marcan como reemplazadas, así queda el porqué.
- **Documentar a medida que se construye**, no al final. Cada fase del roadmap incluye como entregable la documentación que genera.
- Cuando llegue el momento de las guías (usuario, instalación, autoalojamiento, contribución), se organizarán siguiendo el esquema [Diátaxis](https://diataxis.fr/): tutoriales, guías prácticas, referencia y explicación. Lo que escribamos ahora en DECISIONS.md alimentará directamente la parte de «explicación».
