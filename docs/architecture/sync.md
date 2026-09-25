# Sincronización

> Cómo se ponen de acuerdo varios dispositivos sin servidor (Fase 2) y, más adelante, con él (Fase 3). Decisiones de fondo: D05, D13, D25 y D31–D34. Documento vivo: se completa hito a hito.

## Idea

Cada dispositivo guarda todas las operaciones (las suyas y las recibidas). Sincronizar es **intercambiar las operaciones que le faltan al otro** y aplicarlas con unas reglas que hacen que **todos los dispositivos lleguen al mismo estado**, reciban las operaciones en el orden que las reciban. No hay árbitro: las reglas se encargan de ello.

## Qué operaciones se replican (D31)

- Cada operación guarda la **versión del esquema** con la que se registró. Dos dispositivos con versiones distintas no se sincronizan: la app pide actualizar primero. Convertir operaciones antiguas queda para cuando haga falta.
- Las operaciones anteriores a la sincronización (hitos 1.2 a 1.6) quedan como **historial**: se ven en la ficha del libro, pero **no se replican**. Tienen formatos antiguos (la editorial era texto) y ninguna otra copia depende de ellas.
- La primera vez que arranca una versión con sincronización, cada dispositivo registra una **operación inicial** (`sync.baseline`) que crea todo lo que ya tiene. Así viajan también los libros anteriores al registro de operaciones. En una instalación nueva, sin datos, esa operación no existe.
- Consecuencia: **un dispositivo nuevo no necesita una copia completa aparte**. Recibe todas las operaciones replicables, empezando por las iniciales de cada dispositivo.

## Qué le falta a quién: el vector de versiones

Cada dispositivo sabe, para cada autor de operaciones, cuál es la última que tiene: `{dispositivo → última marca HLC}`. Se calcula del registro, sin tablas extra.

Una sincronización entre A y B:
1. A envía su vector a B.
2. B responde con las operaciones que tiene y a A le faltan (las de cada dispositivo posteriores a lo que dice el vector de A), ordenadas por HLC, y con su propio vector.
3. A las aplica.
4. A envía a B las que a B le faltan, según el vector de B. B las aplica.

Se reenvían también las operaciones de terceros: si A sincroniza con B y B con C, lo de A llega a C a través de B.

## Reglas al aplicar operaciones recibidas (D32)

Para cada cambio de cada operación, en orden de HLC:

| Situación | Regla |
|---|---|
| Operación ya conocida (mismo id) | Se ignora: sincronizar dos veces no cambia nada |
| La fila tiene **lápida** (fue borrada) | Se ignora el cambio: **el borrado gana** |
| Crear una fila que no existe | Se crea |
| Crear una fila que ya existe | Se trata como una modificación |
| Modificar | **Campo a campo**: cada campo se escribe solo si la marca de la operación es posterior a la última que escribió ese campo |
| Modificar una fila que no existe | Se ignora (fue borrada en cascada) |
| Borrar | Se borra y se deja una **lápida** con la marca |
| Referencia a una fila que ya no existe (un ejemplar de un libro borrado en otro dispositivo) | Se ignora el cambio |

Para aplicar «campo a campo» cada dispositivo guarda, por cada campo de cada fila, la marca HLC que lo escribió por última vez (tabla `field_versions`). Los borrados quedan en `tombstones`.

Al recibir operaciones, el reloj HLC local salta por delante de las marcas recibidas (`HlcClock::receive`), para que lo que hagas después quede siempre por detrás de lo que ya has visto.

Nada de esto rompe una sincronización: un cambio que no se puede aplicar se salta y se cuenta, pero la operación queda registrada y se sigue reenviando a otros dispositivos.

## Duplicados (D33)

Si en dos dispositivos, sin conexión, se da de alta el mismo autor, la misma editorial, el mismo propietario o el mismo ISBN, al sincronizar **se conservan los dos**. Rechazar uno perdería datos; fusionarlos automáticamente podría fusionar cosas distintas. Por eso:
- Los nombres y el ISBN dejan de ser únicos en la base de datos (siguen siéndolo para lo que tecleas: la app reutiliza lo existente).
- Las listas de sugerencias muestran cada nombre una vez.
- Más adelante habrá una opción para **fusionar duplicados** a mano, que se registrará como una operación más.

## Transporte (D34, hito 2.1)

- **HTTPS por la red local.** El dispositivo que empareja (el escritorio) sirve; el otro se conecta.
- **Certificado propio**, generado la primera vez, con su huella en el QR de emparejamiento. El cliente solo acepta ese certificado: la conexión va cifrada y autenticada sin autoridades de certificación.
- **Emparejamiento:** el QR (o un código para pegar, entre dos escritorios) lleva la dirección, la huella y un secreto de un solo uso. Al emparejar, cada lado guarda al otro con un secreto permanente para las siguientes sincronizaciones.

*(Se detallará en el hito 2.1.)*

## Excepción a «toda escritura es una operación»

Aplicar operaciones recibidas **no genera operaciones nuevas**: se registran las recibidas, tal cual. Es replicación, no una acción del usuario.

## Dónde está en el código (hito 2.0)

- `app/src-tauri/src/sync.rs`: operación inicial, vector de versiones, `operations_since` (lo que le falta al otro) y `merge` (aplicar lo recibido). Sus tests simulan dispositivos con bases de datos en memoria y comprueban que todos acaban con el mismo estado (`assert_converged`).
- `app/src-tauri/src/ops.rs`: las operaciones locales también registran las versiones de campo y las lápidas, así que local y remoto siguen las mismas reglas.
- Pendiente: casos de conformidad de la fusión en `spec/`, para que el servidor PHP (Fase 3) aplique exactamente las mismas reglas.

