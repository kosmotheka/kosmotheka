# La cuenta dentro de la biblioteca (Fase 7, 2.ª tanda) — propuesta para docs/design/errors.md

> Mismo patrón y niveles que la primera tanda: **info** · **aviso** (ámbar, no se perdió nada) · **error** (rojo, la acción no se hizo). Los botones van en negrita. `{lib}` = nombre de la biblioteca, `{nombre}`/`{name}` = persona que administra.

| Código | Nivel | Español | English |
|---|---|---|---|
| `access_revoked_wiped` | info | Ya no estás en «{lib}». Quien la administra te ha retirado el acceso, y la biblioteca se ha borrado de este dispositivo. Si crees que es un error, habla con quien la administra. **Botón:** «Ir a mis bibliotecas». | You’re no longer in “{lib}”. Whoever manages it removed your access, and the library has been deleted from this device. If you think this is a mistake, talk to whoever manages it. **Button:** “Go to my libraries”. |
| `access_revoked_unverified` | aviso | «{lib}» ha dejado de sincronizarse con la nube. La nube dice que ya no tienes acceso, pero no hemos podido comprobarlo. La biblioteca sigue en este dispositivo, y por la red de casa. Habla con quien la administra. | “{lib}” has stopped syncing with the cloud. The cloud says you no longer have access, but we couldn’t confirm it. The library is still on this device and on your home network. Talk to whoever manages it. |
| `upload_admin_only` | info | Esta biblioteca solo puede subirla quien la administra. Pídeselo a {nombre}. Mientras, sigue compartiéndose por la red de casa. | Only whoever manages this library can upload it. Ask {name}. Meanwhile, it keeps sharing over your home network. |
| `device_duplicate` | aviso | Este dispositivo no puede sincronizar «{lib}» con la nube. Hay otro dispositivo registrado igual que este. Es muy raro y no se pierde nada: todo sigue aquí y por la red de casa. Lo arregla quien administra: quitando ese otro dispositivo y vinculando este de nuevo. **Botón:** «Avisar a quien administra». | This device can’t sync “{lib}” with the cloud. Another device is registered exactly like this one. It’s very rare and nothing is lost: everything is still here and on your home network. Whoever manages it can fix it by removing that other device and linking this one again. **Button:** “Tell whoever manages it”. |
| `person_account_taken` | error | Otra cuenta ya está enlazada con {persona} en «{lib}». Cada persona de la biblioteca se enlaza con una sola cuenta. Este dispositivo sigue siendo de {persona} por la red de casa. Si esa otra cuenta no es tuya, habla con quien administra. | Another account is already linked to {person} in “{lib}”. Each person in a library links to one account only. This device still belongs to {person} on your home network. If that other account isn’t yours, talk to whoever manages it. |
| `cloud_link_disabled` | info | La nube no puede dar acceso a este dispositivo en «{lib}». Quien la administra prefiere vincular cada dispositivo a mano. Pídele que vincule este. **Botón:** «Pedir que lo vincule». | The cloud can’t give this device access to “{lib}”. Whoever manages it prefers to link each device by hand. Ask them to link this one. **Button:** “Ask to link it”. |

## Dónde se ven

- `access_revoked_wiped`: pantalla propia al abrir la app (o al volver a ella); después, la lista de bibliotecas sin «{lib}».
- `access_revoked_unverified`, `device_duplicate`, `person_account_taken`, `cloud_link_disabled`: banda arriba del catálogo de esa biblioteca, que no bloquea; la biblioteca sigue usable por la red de casa. Se puede cerrar y vuelve solo si cambia algo.
- `upload_admin_only`: en lugar del botón «Subir» de Compartir, y si el servidor rechaza una subida.

## Textos nuevos que no son errores

| Clave | Español | English |
|---|---|---|
| `cloud_pick.lead` | Las que no subas siguen como hasta ahora, por la red de casa. | The ones you don’t upload keep working as before, over your home network. |
| `cloud_pick.not_admin` | La sube quien la administra | Uploaded by whoever manages it |
| `joined.one` | Ya estás en «{lib}» en la nube. Como {persona} · {rol}. | You’re now in “{lib}” in the cloud. As {person} · {role}. |
| `joined.many` | Ya estás en {n} bibliotecas en la nube · «{lib}», como {persona} · {rol} | You’re now in {n} libraries in the cloud · “{lib}”, as {person} · {role} |
| `device.no_person` | Pide a quien administra que diga de quién es este dispositivo, y entrarás también por la nube. | Ask whoever manages it to say whose device this is, and you’ll also get in through the cloud. |
| `download.title` | Bajando tus bibliotecas de la nube | Downloading your libraries from the cloud |
| `download.finishing` | Terminando de preparar este móvil… | Finishing setting up this phone… |
| `download.done` | Han bajado {n} bibliotecas. En «{lib}» eres {persona} · {rol}. | {n} libraries downloaded. In “{lib}” you’re {person} · {role}. |
| `download.not_uploaded` | Las bibliotecas que no subiste siguen en tus otros dispositivos. | Libraries you didn’t upload are still on your other devices. |
| `autolink.label` | La nube puede dar acceso a dispositivos nuevos | The cloud can give access to new devices |
| `autolink.on` | Encendido: el móvil nuevo de alguien de casa con cuenta entra solo, con el rol de esa persona. | On: a new phone belonging to someone at home with an account gets in on its own, with that person’s role. |
| `autolink.off` | Apagado: cada dispositivo nuevo lo vincula a mano quien administra, como sin nube. | Off: whoever manages the library links each new device by hand, as without the cloud. |
| `device.removed_by_admin` | Quien administra «{lib}» ha quitado este dispositivo. Para volver, pídele que lo vincule otra vez. | Whoever manages “{lib}” removed this device. To come back, ask them to link it again. |
