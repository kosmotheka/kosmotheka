# Guía de lectura para revisar el servidor de la nube

> Para el maintainer, que se comprometió a revisar el código del servidor de 7.2 **todo junto** antes de abrir la nube ([plan de la Fase 7](../plans/fase-7.md)). Escrita el 11/10/2026 por la sesión coordinadora. **No es una revisión: es una ruta.** Dice por dónde entrar, qué mirar en cada fichero y qué preguntarle, para no tener que orientarse en un código que no escribió. Lo que se encuentre se apunta y se decide después, como en una auditoría.

## Antes de leer: verlo funcionar (20 minutos)

Leer este código en frío cuesta. Con los datos delante se entiende mucho antes:

```bash
cd server && make up && make fixtures
```

[fixtures.md](fixtures.md) explica cada escenario (la familia, y **Jhonny**, el vecino hostil que conoce los ids de la familia). Abre la base en tu cliente de siempre y mira tres tablas: `cloud_library`, `membership` y `cloud_operation`. Con eso a la vista, `docs/architecture/cloud.md` se lee de otra manera.

## La idea en cuatro frases

1. **La app manda operaciones firmadas**; el servidor las guarda, comprueba la firma y las vuelve a juzgar con las mismas reglas que la app (D84, D73).
2. **La puerta es clásica:** quién puede tocar una biblioteca lo dice una tabla del servidor (`membership`), nunca un id que mande el cliente.
3. **Sin acceso, es como si no existiera:** `404` igual para «no existe», «no es tuya» y «id mal escrito».
4. **El servidor crea su propio id** para cada biblioteca (`cloud_library.id`, la «C» de los documentos). El id que usa la app solo se compara.

Si al leer un fichero algo contradice una de estas cuatro frases, **eso es un hallazgo**.

## La ruta (unas cuatro horas, en tres sentadas)

### Sentada 1 · La puerta (1 h)

| Orden | Fichero | Líneas | Qué mirar |
|---|---|---|---|
| 1 | `src/Controller/CloudController.php` | 137 | Es fino a propósito. **Cada ruta con `{id}` llama primero a `access->require(...)`.** ¿Hay alguna que no? ¿Alguna usa algo del cuerpo antes de pasar por la puerta? |
| 2 | `src/Sync/CloudLibraryAccess.php` | 113 | El corazón de la puerta. `require()` son cinco líneas: léelas despacio. ¿Las tres primeras salidas dan de verdad la misma respuesta? `isCut()` es lo más enrevesado del fichero: decide cuándo alguien que tenía acceso deja de tenerlo. |
| 3 | `src/Entity/Membership.php`, `CloudLibrary.php` | 106 + 140 | Qué guarda cada una. ¿Hay algún modificador (`set…`) que no debería poder llamarse desde fuera? |
| 4 | `src/Repository/MembershipRepository.php` | 45 | Todas las consultas filtran por cuenta **y** biblioteca. ¿Alguna busca solo por un id? |
| 5 | `tests/Controller/CloudSyncControllerTest.php`, solo los nombres de los tests | 1.838 | No lo leas entero: lee la lista de `public function test…`. Es el índice de lo que se defiende. ¿Echas en falta algún caso que a ti se te ocurra? |

**Pregunta de la sentada:** si yo fuera Jhonny, con una cuenta válida y los ids de mi vecino, ¿qué línea me para en cada ruta?

### Sentada 2 · Lo que entra (1,5 h)

| Orden | Fichero | Líneas | Qué mirar |
|---|---|---|---|
| 6 | `docs/architecture/cloud.md`, «Reglas de `push`» | — | Las diez reglas, antes del código. |
| 7 | `src/Sync/CloudPush.php` | 403 | El fichero más importante. Tres métodos: `push()` (la transacción y los avisos), `applyLocked()` (el bucle) y **`accept()`**, que es donde cada regla se convierte en un `throw`. Ve regla por regla del documento buscando su `throw`. |
| 8 | `src/Sync/CloudLink.php` | 184 | Cómo se enlaza una biblioteca recién creada con la de la app. Es la única operación que se verifica con la clave **de la cuenta**, no de la biblioteca. Es el punto donde más arreglos ha habido (SEG-013, SEG-022, SEG-024): léelo con desconfianza. |
| 9 | `src/Sync/OperationSignature.php` | 110 | Qué bytes exactos cubre la firma. ¿Hay algún campo de la operación que se guarde y **no** esté firmado? |
| 10 | `src/Sync/OperationReader.php`, `src/Dto/OperationPayload.php` | 85 + 57 | Cómo se lee lo que manda el cliente. ¿Qué pasa con un campo de más, uno de menos, un texto enorme? |
| 11 | `src/Repository/CloudOperationRepository.php` | 434 | SQL a mano (aquí sí, por rendimiento). Mira que **todo valor vaya como parámetro** y que toda consulta lleve la biblioteca. Es largo: basta con recorrer cada consulta. |

**Pregunta de la sentada:** ¿puede entrar una operación con el nombre de otro dispositivo, o entrar en una biblioteca que no es la suya?

### Sentada 3 · Lo que sale, y las cuentas (1,5 h)

| Orden | Fichero | Líneas | Qué mirar |
|---|---|---|---|
| 12 | `src/Sync/CloudPull.php` | 41 | Corto. ¿Devuelve algo que no sea de esa biblioteca? ¿Tiene tope? |
| 13 | `src/Controller/AuthController.php`, `AccountController.php` | — | La entrada de todas las cuentas: el reto firmado, los límites, que no se diga si un correo existe. [accounts.md](../architecture/accounts.md) lo explica. |
| 14 | `src/Security/DeviceTokenAuthenticator.php`, `DeviceProof.php` | — | Cómo se reconoce a un dispositivo en cada llamada. ¿El token se guarda en claro o solo su huella? ¿Se compara en tiempo constante? |
| 15 | `config/packages/security.yaml`, `rate_limiter.yaml` | — | Qué rutas son públicas (deberían ser muy pocas) y qué límites hay. |

**Pregunta de la sentada:** ¿qué puede hacer alguien **sin** cuenta? ¿Y con una cuenta recién creada y nada más?

## Lo que no hace falta leer en esta pasada

- `src/Sync/Merge/` y `src/Sync/Permission/` (unas 2.000 líneas con `AccessCutter`, `AccountLinker` y `CloudAuthor`): el motor que fusiona y juzga. Corre contra los mismos casos compartidos que la app (`spec/conformance/`, 121 casos), así que su comportamiento está atado por otro lado. Son de 7.4 y 7.5, no de 7.2.
- Invitaciones y registro de actividad (7.6 y 7.7): auditados el 11/10/2026 ([informe](../audits/2026-10-11.md)).
- Los tests, salvo sus nombres.

## Cómo apuntar lo que veas

Una línea por cosa, sin arreglar nada mientras lees:

```
fichero:línea · qué he visto · por qué me chirría · ¿duda o fallo?
```

Tres tipos de nota valen igual: «esto está mal», «esto no lo entiendo» (si tú no lo entiendes con tu experiencia en Symfony, está mal explicado) y «esto es más complicado de lo que debería». La tercera es la que menos encuentran las auditorías automáticas y la que más aporta tu lectura.

## Lo que ya se sabe, para no apuntarlo dos veces

Los hallazgos abiertos están en [audits/hallazgos.md](../audits/hallazgos.md). De este código, los que siguen abiertos son bajos: dos `catch` vacíos comentados (CAL-004), el token de la invitación en la ruta (SEG-046). Las rutas `pull`, `push` y `claim` **no tienen límite de peticiones**: está apuntado para la revisión de seguridad de conjunto.
