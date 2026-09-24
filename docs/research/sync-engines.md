# Motores de sincronización existentes: qué aprender de ellos

> Notas de investigación para D05. No vamos a usar ninguno, pero su diseño nos ahorra errores.
> Datos escritos de memoria (conocimiento hasta mediados de 2026): **verificar licencias y estado actual antes de citarlos en documentación pública.**

## PowerSync

- **Qué es:** un servicio que se coloca junto a tu PostgreSQL (también admite MySQL y MongoDB) y mantiene una copia en SQLite en cada cliente.
- **Cómo lee:** escucha los cambios de PostgreSQL mediante la *replicación lógica* (el mismo mecanismo que usan las réplicas de base de datos). Unas reglas en el servidor deciden qué datos recibe cada usuario, agrupados en *buckets* (por ejemplo, «todo lo de la biblioteca X»).
- **Cómo escribe:** el cliente escribe en su SQLite, y cada cambio entra en una **cola de subida**. La app envía esa cola **a tu propio backend**, que decide si aplica cada cambio. PowerSync no aplica escrituras por su cuenta.
- **SDK:** Flutter, React Native, JavaScript, Kotlin, Swift y .NET. No recuerdo un SDK oficial para Rust (a verificar).
- **Licencia:** el servicio tiene una edición autoalojable con una licencia de tipo *source-available*, que no es open source en sentido estricto (a verificar).

## ElectricSQL

- **Qué es:** un motor, escrito en Elixir, que sincroniza **subconjuntos** de tablas de PostgreSQL con los clientes, a través de HTTP normal y cacheable.
- **Conceptos:** las ***shapes*** son la definición de un subconjunto («los ejemplares de la biblioteca X»). El cliente se suscribe a una *shape* y recibe primero una copia completa y después los cambios.
- **Cómo escribe:** en su versión actual, **no gestiona las escrituras**. Las haces contra tu propia API, igual que en PowerSync. En 2024 abandonaron su primer diseño, que sí sincronizaba escrituras de forma automática, por ser demasiado complejo.
- **Licencia:** Apache-2.0 (a verificar).

## Otros que merece la pena conocer

- **Replicache / Zero** (Rocicorp): el origen del patrón que hemos elegido. El cliente aplica mutaciones de forma optimista y el servidor las vuelve a ejecutar con autoridad.
- **cr-sqlite**: una extensión de SQLite que convierte las tablas en CRDT y permite fusionar bases de datos sin servidor. Es la idea más cercana a D13. Parecía poco mantenida la última vez que la revisé.
- **Automerge / Yjs**: CRDT de documentos. Encajan mejor en editores colaborativos que en un catálogo relacional.

## Ideas que nos llevamos

1. **Escrituras por tu API y lecturas sincronizadas.** Los dos motores principales llegaron por separado al diseño de D05: las lecturas se sincronizan de forma automática y las escrituras pasan por tu backend, que aplica sus reglas. Es una buena señal de que el diseño es sensato.
2. **Cola de subida (patrón *outbox*).** Guardar las operaciones pendientes en una tabla local, no en memoria, para que sobrevivan a un cierre de la app o a un corte de red.
3. **Réplica parcial.** Cada cliente recibe solo lo que le corresponde. Para nosotros, al principio, «la biblioteca de mi familia», pero conviene que el protocolo lo contemple desde ya, pensando en la comunidad.
4. **Cursor de sincronización monótono.** El cliente dice «tengo todo hasta el punto N» y el servidor le envía lo posterior. Es simple, se puede reanudar y es fácil de depurar.
5. **Copia inicial y cambios después.** Un cliente nuevo recibe primero una foto completa y luego los cambios. Es el mismo esquema que la v1 y la v2 de D13.
6. **Aprender de lo que abandonó ElectricSQL.** Sincronizar escrituras de forma automática y genérica es muy difícil. Operaciones explícitas y con nombre («prestar ejemplar») son más fáciles de validar, de versionar y de entender.
