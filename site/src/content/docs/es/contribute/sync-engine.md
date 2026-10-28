---
title: El motor de sincronización, explicado
description: Cómo varios dispositivos con los mismos libros llegan al mismo resultado sin que nadie mande, y por qué se diseñó así.
sidebar:
  order: 6
---

Esta es la página central de la parte técnica. Explica cómo Kosmotheka consigue que dos dispositivos que cambian el mismo libro, sin conexión, acaben viendo **exactamente lo mismo**. Si no conoces el proyecto, léela de principio a fin: va de lo sencillo a lo difícil, y cada término se explica la primera vez que sale. Los nombres que se repiten en el código están en el [glosario](../glossary/).

## La idea, en cuatro frases

1. Cada dispositivo guarda **todas las operaciones**: las suyas y las que ha recibido. Una operación es un lote de cambios con una etiqueta de intención, como «añadir este libro».
2. Sincronizar es **darse unos a otros las operaciones que les faltan**.
3. Cada dispositivo aplica las operaciones con **unas reglas que no dependen del orden** en que llegan.
4. Por eso todos acaban en el mismo estado, **sin árbitro**: no hace falta un dispositivo ni un servidor que decida.

El resto de la página explica cada pieza y por qué hace falta.

## Un ejemplo que seguiremos

Ana y Hugo comparten la biblioteca «Casa». Ana tiene el móvil; Hugo, el portátil. Los dos tienen *Dune*, con un ejemplar en tapa blanda (en el salón, una unidad), y también *Rayuela*. Ahora los dos se quedan **sin conexión** (cada uno en su sitio) y hacen cosas:

| Cuándo (hora de su reloj) | Quién | Qué hace |
|---|---|---|
| 10:00 | Ana | Cambia la ubicación del ejemplar a «Estantería 2» y escribe una nota: «prestado una vez» |
| 10:05 | Hugo | Cambia la ubicación del mismo ejemplar a «Despacho» y pone 2 unidades |
| 10:10 | Hugo | Borra la edición de otro libro, *Rayuela* |
| 10:20 | Ana | Cambia el año de *Rayuela*, sin saber que Hugo ya la ha borrado |

Más tarde se conectan. ¿Qué debe pasar? Ana y Hugo tienen que ver lo mismo. Lo razonable es:

- La ubicación del ejemplar de *Dune* es una sola, no las dos. Gana «Despacho», porque Hugo la cambió después.
- Pero **no se pierde nada más**: la nota de Ana y las dos unidades de Hugo se quedan, porque cambiaron campos distintos.
- *Rayuela* está borrado para los dos, aunque Ana lo editara después.

Veamos cómo lo consiguen las piezas.

## 1. Operaciones y registro

Ninguna parte de la app escribe directamente en las tablas del catálogo. Cada acción («guardar la ficha») se convierte en una **operación**: un lote atómico de cambios (crear, modificar o borrar una fila de una entidad) con una etiqueta de intención como `book.add`. La operación se apunta en un **registro** (la tabla `operations`) y sus cambios se aplican **en la misma transacción**: o ocurre todo, o nada.

*Por qué así:* con las operaciones, sincronizar es solo intercambiar operaciones, el servidor puede comprobar los permisos sobre sus cambios, y el historial de cada libro sale gratis del registro. La forma exacta está en [Las operaciones](../operations-spec/).

En el ejemplo, la operación de Ana lleva dos cambios sobre una fila de ejemplar: su ubicación y su nota. La de Hugo, otros dos: ubicación y unidades.

## 2. El reloj lógico híbrido: poner orden sin un reloj común

Para decidir quién cambió algo «después», hace falta ordenar las operaciones. Pero los relojes de los dispositivos no coinciden: uno puede ir adelantado, o alguien puede cambiar la hora. Un **reloj lógico híbrido** (HLC, por *hybrid logical clock*) resuelve esto. Cada operación lleva una marca con tres partes:

- los **milisegundos** del reloj del dispositivo;
- un **contador**, que desempata si hay dos operaciones en el mismo milisegundo o el reloj retrocede;
- el **identificador del dispositivo**, que desempata entre dispositivos distintos.

Se escribe con ceros a la izquierda, por ejemplo `0001727222222222-00000003-<id del dispositivo>`, de modo que ordenar las marcas como texto es lo mismo que ordenarlas en el tiempo.

Dos propiedades importan:

- **La marca de un dispositivo nunca retrocede**, aunque el reloj del sistema lo haga.
- **Al recibir operaciones, el reloj salta por delante de lo recibido.** Así, lo que hagas después queda siempre «después» de lo que ya has visto, aunque el reloj del otro dispositivo fuera adelantado.

Y un límite de seguridad: una operación cuya hora va **más de una hora por delante** del reloj de quien la recibe se rechaza. Sin él, un dispositivo con el reloj en el año 2099 ganaría todos los conflictos para siempre.

Como cada marca incluye el identificador del dispositivo, **dos operaciones distintas nunca tienen la misma marca**. Una app honrada no firma dos con la misma; si llegan dos, la segunda se rechaza.

## 3. La fusión: campo a campo, y gana la marca más alta

Cuando llega una operación, cada dispositivo aplica sus cambios con estas reglas:

| Situación | Qué se hace |
|---|---|
| La operación ya se conocía (mismo id) | Se ignora. Sincronizar dos veces no cambia nada |
| Modificar una fila | **Campo a campo**: cada campo se escribe solo si la marca de la operación es más alta que la de quien lo escribió por última vez |
| Crear una fila que ya existe | Se trata como una modificación |
| Modificar una fila que no existe | Se ignora (se había borrado) |
| Borrar | Se borra y se deja una **lápida** (ver más abajo) |
| Cambiar una fila que tiene lápida | Se ignora: **el borrado gana** |

Para poder decidir «campo a campo», cada dispositivo guarda, por cada campo de cada fila, la marca de la operación que lo escribió por última vez (la tabla `field_versions`).

**En el ejemplo.** El ejemplar tiene tres campos tocados: ubicación (10:00 Ana, 10:05 Hugo), nota (solo Ana) y unidades (solo Hugo). La ubicación la gana la marca de las 10:05: «Despacho». La nota y las unidades, que nadie más tocó, se quedan. Es lo que queríamos, y se consigue **sin que los dos dispositivos se hablen** más allá de intercambiar operaciones.

*Por qué «campo a campo» y no «la fila entera»:* si ganara la fila entera, la operación de Hugo borraría la nota de Ana, que él ni conocía.

### Por qué siempre converge

Es lo importante, y tiene una explicación corta. Cada regla anterior cumple tres cosas:

- el resultado **no depende del orden** en que llegan las operaciones (para cada campo gana la marca más alta, entre las que lleguen primero o después);
- **repetir una operación no cambia nada** (se ignora si ya se conocía);
- **no hay empates**, porque cada marca es única.

Un sistema así termina igual, reciba lo que reciba y en el orden que sea. Y no solo se razona: los tests de convergencia simulan varios dispositivos, entregan las mismas operaciones en órdenes distintos y comprueban con `assert_converged` que todos acaban iguales. Además comprueban que **recalcular la biblioteca entera desde el registro** (ver más abajo) la deja exactamente igual.

## 4. Las lápidas: que un borrado no resucite

Si Hugo borra *Rayuela* y Ana, sin saberlo, lo edita después, la edición de Ana no debe devolverlo a la vida. Por eso un borrado deja una **lápida**: una marca que dice «esta fila se borró» y con qué marca. Un cambio posterior sobre una fila con lápida se ignora. **El borrado gana siempre.**

*Por qué así:* la alternativa es perder el borrado cuando otro dispositivo toca la fila a la vez. Aceptamos que alguien pierda una edición a cambio de que un borrado no pueda deshacerse solo.

Algunos casos con dependientes: al borrar una obra se borran, en cadena, sus ediciones y los ejemplares de esas ediciones. Una referencia **opcional** a algo borrado (la editorial de una edición, el dueño de un ejemplar) se deja vacía, en vez de rechazar el cambio. Una referencia **obligatoria** a algo que ya no existe hace que el cambio se ignore, porque la fila habría desaparecido en cascada de todos modos.

**En el ejemplo.** La edición de *Rayuela* tiene lápida desde las 10:10; la de Ana es de las 10:20 y se ignora. Los dos acaban sin *Rayuela*.

### La papelera, antes del borrado de verdad

Desde [D105](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md), «borrar» en la app no borra: una ficha, un ejemplar, una etiqueta o una saga pasan a la **papelera**, que son dos campos más de la fila, `trashed_at` (cuándo) y `trashed_by` (qué persona). Restaurar es ponerlos a `null`. Para el motor es un cambio de campos como cualquier otro, así que se fusiona igual: gana la marca más alta, y lo que cuelga de la fila no se toca, por eso vuelve entero. Si Hugo manda *Rayuela* a la papelera y Ana la edita sin saberlo, la edición de Ana se guarda, y al restaurar vuelve con ella.

El borrado de verdad, con su lápida, es **eliminar definitivamente**: lo hace quien administra cuando quiere, y cualquier editor o administrador cuando la cosa lleva 30 días en la papelera. Si una persona restaura mientras otra elimina, gana eliminar, como siempre.

Esos 30 días los cuenta cada dispositivo desde **la más tardía de dos fechas**: `trashed_at` y la primera vez que **él** vio esa fila en la papelera, apuntada en una tabla local que no se sincroniza (`trash_seen`). Un dispositivo con el reloj adelantado no hace así que los demás eliminen antes de tiempo, y una copia de seguridad antigua da 30 días más a lo que traiga en la papelera. Si dos dispositivos eliminan lo mismo, el segundo no cambia nada. El servidor hace lo mismo a los 60 días, como red de seguridad.

## 5. Cómo se piden lo que les falta: el vector de versiones

¿Cómo sabe un dispositivo qué operaciones le faltan al otro? Cada dispositivo calcula, para cada biblioteca y cada autor de operaciones, **cuál es la última marca que tiene**: es su **vector de versiones**. Se obtiene del registro, sin tablas extra. Una sincronización entre A y B:

1. A envía su vector a B.
2. B responde con las operaciones que A no tiene (las de cada autor posteriores a lo que dice el vector de A) y con su propio vector.
3. A las aplica.
4. A envía a B las que a B le faltan, según el vector de B.

También se reenvían las operaciones de terceros: si A sincroniza con B y B con C, lo de A llega a C a través de B. Así un móvil hace de puente entre otro móvil y el portátil.

**Cómo hablan:** por la red de casa, tras emparejarse con un código QR. El QR contiene un secreto de un solo uso y la huella del certificado del otro dispositivo; así cada uno reconoce al otro y la conexión va cifrada, sin contraseñas. Los datos no viajan por el QR (no caben), viajan por la red local.

## 6. Las firmas y la cadena de cada autor

Hasta aquí todo supone dispositivos honrados. Pero un dispositivo puede estar modificado, o un compañero puede intentar hacerse pasar por otro. Hay dos defensas en el propio formato de la operación.

**Firma.** Cada dispositivo tiene una clave secreta que nunca sale de él, y **firma cada operación** que hace. Los demás conocen su clave pública, que viaja con la biblioteca, y comprueban la firma al recibirla. Una operación sin firma, o con una que no cuadra, se descarta. La clave de un dispositivo no cambia nunca: si llega otra distinta para el mismo dispositivo, se rechaza.

**Cadena.** Cada operación lleva un campo `previous` (anterior): el id de la **anterior operación del mismo dispositivo en esa biblioteca**, o nulo si es la primera. Va firmado con el resto. Esto hace dos cosas:

- Quien recibe una operación **sabe si le falta una anterior**. Si la `previous` no está en su registro, la operación **espera**, junto con las siguientes de su autor, y la próxima sincronización la pedirá con todas las que faltan. Nunca se salta una operación que falta, así que el vector de versiones no puede dejar un hueco sin que nadie lo vea.
- Nadie puede **intercalar** una operación nueva en medio de las antiguas de un dispositivo sin que se note (se verá más abajo por qué importa).

## 7. Los permisos se comprueban al recibir

Cada operación recibida se juzga con el **rol que tenía su autor en ese momento**: administrador, editor o lector. Si todos sus cambios están permitidos, se aplica. Si alguno no lo está, **la operación entera se guarda como petición**: se queda en el registro y viaja a los demás dispositivos, pero no se aplica, hasta que quien administra la acepte o la rechace. Es «todo o nada», y nada se pierde.

Hay un caso difícil. Si a Hugo le quitan el rol de editor mientras su portátil está sin conexión, lo que haga después **deja de valer**. Pero eso puede llegar a otro dispositivo *después* de operaciones que Hugo ya hizo. Cuando algo llega con una marca más antigua que lo ya aplicado, el dispositivo **recalcula la biblioteca desde su registro**: vacía su contenido, vuelve a aplicar todas las operaciones en orden de marca y juzga cada una con los roles de su momento. Así todos llegan al mismo resultado, en el orden que sea.

Hace falta una protección más: la marca de una operación la pone quien firma. Si a Hugo le quitan el rol hoy, con una app modificada podría firmar un cambio y ponerle la fecha de la semana pasada, cuando aún era editor. Contra eso, quien quita poder anota en esa misma operación **hasta qué operación del dispositivo cree** (la última que había visto, por su `previous`). Es una **valla**: lo que aparezca de ese dispositivo con fecha anterior y no cuelgue de esa operación no se cree, queda como petición. Y una biblioteca nace ya con su administrador (la **fundación**), de modo que no existe un momento anterior en que no hubiera roles y se pudiera fechar hacia atrás.

El modelo completo de amenazas está en [Seguridad: el modelo de amenazas](../security/).

## 8. Qué hace el servidor, cuando lo hay

Si alguien sube una biblioteca a un servidor, **el servidor es un dispositivo más con una diferencia: es la autoridad**. Cuando le llega una operación:

1. Comprueba que está bien formada (la misma validación que la app, con los mismos casos de prueba).
2. Comprueba la **firma** del dispositivo que la hizo.
3. Comprueba los **permisos** con las mismas reglas que los dispositivos.
4. Si todo está bien, la aplica sobre PostgreSQL con **el mismo motor de fusión**, escrito otra vez en PHP, y avisa a los demás dispositivos.

Cada dispositivo pide al servidor lo que le falta con el mismo vector de versiones. El servidor **nunca se fía de un identificador que mande el cliente**: lo compara con lo que él sabe, y el acceso sale de la cuenta y de su propia lista de quién entra en qué biblioteca. Las reglas están escritas una vez, en `spec/`, y las ejecutan los mismos casos de conformidad en Rust y en PHP.

Sin servidor, todo lo anterior sigue funcionando entre los dispositivos emparejados. El servidor añade que la biblioteca se ponga al día aunque los dispositivos no coincidan nunca en la misma red.

## Por qué así y no de otra forma

Esta es la valoración del proyecto, no un estudio comparativo: dice por qué descartamos otras maneras para un catálogo familiar con roles.

**Un servidor central único.** Es lo más sencillo: el servidor decide y los clientes lo consultan. Lo descartamos como única opción porque la app tiene que funcionar **sin cobertura** (en una tienda, mirando «¿lo tengo ya?») y **sin cuenta**: para mucha gente, la mejor privacidad es que no haya servidor. Por eso la app escribe siempre en su propia base y el servidor es opcional. Pero conservamos de esta idea lo que sí hace falta: cuando hay servidor, es la autoridad y comprueba los permisos.

**Sincronizar archivos** (copiar la base de datos o un archivo exportado entre dispositivos, o usar un servicio de carpetas compartidas). Es fácil de entender, pero cuando dos dispositivos cambian a la vez, gana **un archivo entero** y se pierde lo del otro, o se crean copias en conflicto que hay que juntar a mano. Una base de datos abierta tampoco se copia de forma segura mientras se escribe. Y no hay forma de decir quién puede cambiar qué.

**Conectar los dispositivos por internet con un relé** (un servicio por el que pasa el tráfico entre ellos, por ejemplo con WebRTC). Permite sincronizar fuera de casa sin abrir puertos, pero exige que **alguien mantenga ese relé**, que ve quién habla con quién y cuándo. En casa no hace falta: dos dispositivos en la misma red se hablan directamente, con un QR y sin nadie en medio. Fuera de casa está el servidor opcional, que además entiende los datos y puede comprobar permisos.

**CRDT de documento** (estructuras como Automerge o Yjs, pensadas para que varias personas editen el mismo texto a la vez). Garantizan la convergencia sin árbitro, y la idea de que una estructura converja por construcción es la que hemos seguido. Pero están pensadas para documentos, no para un catálogo relacional con roles, y es difícil imponer permisos («que un lector no borre») cuando cada copia puede decidir por su cuenta. Hemos tomado de ellos lo útil, que es la convergencia, con piezas más pequeñas: marcas por campo, lápidas y operaciones explícitas.

**Un motor de sincronización ya hecho** (hay varios que sincronizan PostgreSQL con SQLite en los clientes). Dos cosas nos hicieron elegir uno propio: esos motores parten de un servidor (las escrituras pasan por tu propio servidor), y queríamos que dos móviles se sincronizaran sin ninguno; y que uno de los objetivos del proyecto es entender cómo funciona. Sí aprendimos de ellos: guardar las operaciones pendientes en una tabla local, y pedir «lo posterior a mi punto» en vez de volver a copiarlo todo.

**Operaciones con nombre propio** (una operación distinta para «prestar», otra para «borrar»…), en vez de lotes de cambios con una etiqueta. Cada tipo definiría su propia fusión, y habría que escribirla dos veces (Rust y PHP). Con cambios genéricos, la fusión se escribe una vez para todo, y los permisos se comprueban sobre los **cambios**, no sobre la etiqueta: nadie puede colar un cambio bajo un nombre inocente.

Las decisiones que hay detrás, con sus alternativas y fechas, están en el [registro de decisiones](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md): D05 (local primero, servidor autoritativo), D13 (sincronización local por QR), D25 (forma de las operaciones), D31 (versiones del formato), D84 (acceso a la nube), D87 (la cadena de cada autor).

## Qué no hace

Para que nadie lo dé por supuesto:

- **Dos cambios a la vez en el mismo campo: gana uno.** El otro se queda en el registro, pero deja de verse. Es la regla «último en escribir gana», sin avisar.
- **Los duplicados no se juntan solos.** Si dos dispositivos dan de alta, sin conexión, el mismo autor, la misma editorial o el mismo ISBN, se conservan los dos: rechazar uno perdería datos, y fusionarlos a ciegas podría juntar cosas distintas.
- **Dos dispositivos con versiones muy distintas de la app no se sincronizan.** La app pide actualizar primero.
- **Lo ya entregado no se puede recuperar.** Si se quita el acceso a un dispositivo, conserva lo que ya tenía copiado.
- **Un editor con una app modificada puede bifurcar su propia cadena** (dos operaciones con la misma anterior) y que solo la reciban los dispositivos que iban por detrás. La cadena lo hace detectable, pero no se resuelve: es un riesgo aceptado, limitado a quien ya podía editar y a lo que ya podía hacer.

## Dónde está en el código

- `app/src-tauri/src/sync.rs`: el vector de versiones (`version_vectors`), lo que le falta al otro (`operations_since`) y la fusión (`merge`, `apply_change`), con los tests que simulan dispositivos.
- `app/src-tauri/src/ops.rs`: las operaciones locales, que usan las mismas reglas.
- `app/src-tauri/src/trash.rs`: la papelera (mandar, restaurar, eliminar y lo que caduca a los 30 días).
- `crates/domain/src/hlc.rs`, `operation.rs`, `permission.rs`, `signature.rs`: el reloj, la validación, los permisos y las firmas, sin base de datos.
- `app/src-tauri/src/lan/`: emparejamiento, certificado y transporte por la red local.
- `server/src/Sync/`: el servidor; `Sync/Merge/` es el motor de fusión.
- `spec/conformance/`: los casos que comparten Rust y PHP.

Siguiente: [las operaciones y la especificación](../operations-spec/).
