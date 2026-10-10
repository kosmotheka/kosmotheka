# Encargo de diseño: la cuenta dentro de la biblioteca (Fase 7, segunda tanda)

> Encargo para Claude Design, preparado el 10/10/2026. Completa el [primer encargo de la Fase 7](encargo-fase-7-cuentas.md), cuyas pantallas están en [`screens/cloud-account/`](screens/cloud-account/). Se sube como guía al proyecto de Design y se pega tal cual lo que va en el bloque. Se basa en D76 (con su enmienda del 10/10/2026), D71 y D96 ([DECISIONS.md](../DECISIONS.md)) y en lo construido en el hito 7.5 ([architecture/security.md](../architecture/security.md), [architecture/cloud.md](../architecture/cloud.md)).

```
Encargo: Fase 7, segunda tanda — La cuenta dentro de la biblioteca (Kosmotheka)

Contexto
En la primera tanda diseñasteis la cuenta y subir bibliotecas a la nube. Desde entonces hemos construido cómo entra cada persona de la familia en una biblioteca que ya está en la nube, y han cambiado dos cosas. Partid de vuestras pantallas CloudPick, Sharing, Settings y de las de personas y roles. Escritorio y móvil, claro y oscuro, textos en español e inglés.
Tono: la app habla de personas y de móviles, nunca de claves, firmas ni identificadores. Si algo no está claro, decidid lo más sencillo y listad al final lo que hayáis supuesto.

Lo que ha cambiado respecto a la primera tanda
- Solo sube una biblioteca a la nube quien la administra. Antes decíamos "quien puede compartir". En CloudPick, las bibliotecas que no administras salen sin casilla, con una línea que lo explica ("La sube quien la administra"), y el texto de cabecera deja de decir "solo las que puedes compartir".
- En una biblioteca sin personas ni roles (una sola persona, o una pareja que lo comparte todo) cualquiera puede subirla, como hasta ahora.

1. Entrar en una biblioteca de la nube sin que nadie te invite
Situación: Jhonny ya tiene la biblioteca familiar en su móvil, por la red de casa, y quien administra ya dijo "este móvil es de Jhonny". Jhonny crea su cuenta o inicia sesión. Sin hacer nada más, queda dentro de esa biblioteca en la nube, con su mismo rol.
- Un aviso discreto que lo cuente, sin pedir nada: qué biblioteca, como quién y con qué rol ("Ya estás en «Casa» en la nube, como Jhonny · lector").
- Si son varias bibliotecas, un solo aviso que las liste.
- Si su móvil todavía no es de nadie en esa biblioteca, no pasa nada y no se avisa: en la biblioteca, el texto que ya existe para un dispositivo sin persona ("Pide a quien administra que diga de quién es este dispositivo") basta. Proponed si hace falta añadir "…y entrarás también por la nube".

2. El móvil nuevo
Situación: Laura estrena móvil, instala la app e inicia sesión con su cuenta. Sus bibliotecas de la nube bajan solas y, al terminar, el móvil es "de Laura" con su rol, sin que nadie en casa haga nada.
- Lo que ve mientras baja (puede tardar con bibliotecas grandes) y al terminar: "Han bajado 2 bibliotecas. En «Casa» eres Laura · administradora".
- Un momento intermedio corto en el que la biblioteca ya está pero el móvil aún no tiene su rol: que no parezca un error ni deje editar para luego rechazarlo. Una línea de "Terminando de preparar este móvil…" basta.
- Lo que no está en la nube no baja: una línea que lo recuerde sin alarmar ("Las bibliotecas que no subiste siguen en tus otros dispositivos").

3. "La nube puede dar acceso a mis dispositivos nuevos"
Es el interruptor que ya dibujasteis en CloudPick. Ahora existe de verdad y hay que poder cambiarlo después.
- En los ajustes de cada biblioteca que está en la nube, solo para quien la administra: el interruptor, con una explicación de dos líneas de qué hace encendido (un móvil nuevo de alguien con cuenta entra solo, con el rol de esa persona) y apagado (cada dispositivo nuevo lo vincula a mano quien administra, como sin nube).
- Apagarlo pide confirmar y dice qué pasa: los dispositivos ya vinculados siguen igual; los nuevos, a mano.
- Para quien no administra: se ve el estado, no se puede cambiar.

4. Este dispositivo es de otra persona
Situación: Laura inicia sesión con su cuenta en la tablet de casa, que en la biblioteca figura como "de Hugo". La app no cambia nada sola: pregunta.
- Un diálogo que lo explique en una frase y ofrezca: "Dejarlo como está" (la tablet sigue siendo de Hugo; Laura ve la biblioteca con el rol de Hugo en este dispositivo) o "Pedir a quien administra que lo cambie". Si Laura administra, puede cambiarlo ahí mismo.
- No debe volver a preguntar cada vez que se abre la app.

5. Quién tiene cuenta
- En la lista de personas de la biblioteca, una marca discreta en quien tiene cuenta enlazada, sin mostrar su email. Sirve para saber a quién no hace falta invitar.
- En la ficha de una persona, la lista de sus dispositivos distingue los que vinculó alguien a mano de los que entraron por la nube (una etiqueta pequeña: "por la nube").

6. Quitar un dispositivo o a una persona
- Quitar un dispositivo a su persona (por ejemplo, un móvil perdido) cuando la persona tiene cuenta: el aviso dice que ese móvil deja de tener su rol y que la nube no se lo devolverá sola aunque siga con la sesión abierta; y recomienda, además, que su dueño lo quite de "Mis dispositivos" en su cuenta o cambie la contraseña.
- Lo que ve ese dispositivo: "Quien administra «Casa» ha quitado este dispositivo. Para volver, pídele que lo vincule otra vez."
- Echar a una persona de la biblioteca (alguien que se va de la familia): deja de poder entrar, ni con sus dispositivos ni con su cuenta, al momento. Su nombre sigue en la biblioteca: sus libros siguen siendo suyos y sus préstamos y lo que marcó se conservan. El aviso lo dice, y recuerda que lo que ya tenía copiado fuera de la app no se le puede quitar. En la lista de personas se ve como "sin acceso", con "Devolver el acceso" para quien administra (que después tiene que vincular de nuevo un dispositivo suyo). No se puede echar al último administrador.
- Salir yo de una biblioteca: como hasta ahora, se va solo de este dispositivo; mis otros dispositivos la conservan y la nube no la vuelve a bajar aquí. Ofrece una copia antes.

7. Errores y estados (para docs/design/errors.md)
Proponed el texto, en español e inglés, de:
- Te han retirado el acceso a esta biblioteca y se ha borrado de este dispositivo (la app lo ha comprobado con la orden de quien administra).
- La nube dice que ya no tienes acceso a esta biblioteca, pero la app no ha podido comprobarlo: la biblioteca sigue en este dispositivo y ha dejado de sincronizarse con la nube. Habla con quien la administra.
- Esta biblioteca solo puede subirla quien la administra.
- Este dispositivo no puede sincronizar esta biblioteca con la nube: hay otro dispositivo registrado con su mismo nombre interno. No se pierde nada; lo arregla quien administra quitando ese dispositivo y vinculando este de nuevo. (Es un caso muy raro; el texto debe tranquilizar y decir a quién acudir.)
- Otra cuenta ya está enlazada con esta persona en esta biblioteca.
- La nube no tiene permiso para vincular dispositivos nuevos en esta biblioteca: pide a quien administra que vincule este.

Qué entregar
Las pantallas nuevas y las que cambian (CloudPick, ajustes de la biblioteca, personas), móvil y escritorio, claro y oscuro; los textos en los dos idiomas; y la lista de lo que hayáis supuesto.
```

## Notas para quien construya las pantallas (hito 7.5.5)

- El punto 6 ya está construido por dentro (hitos 7.5.4a y 7.5.4b, D98): `people::withdraw_person` y `restore_person` existen sin comando de Tauri ni pantalla; los avisos `cloud.notice.removed` y `accessWithdrawn` ya tienen texto provisional.
- La etiqueta «por la nube» del punto 5 necesita saber quién firmó la vinculación de cada dispositivo; hoy la app no lo expone a las pantallas.
- El caso del punto 4 lo devuelve el servidor en `conflicts` al reclamar (`POST /v1/cloud/libraries/claim`); la app todavía no lo guarda ni lo enseña.
- El texto de `cloud.pick.intro` («Solo las que puedes compartir») ya es falso desde 7.5.2b-1 y se corrige al construir estas pantallas.
