# Propuesta: nombres de las cuatro formas de «quitar» un aparato (17/10/2026)

> Fase 0, **sin aplicar nada**. El maintainer decide antes de cambiar una sola cadena. Lo que hace cada una está mirado en el código (`app/src-tauri/src/lan/mod.rs`, `people.rs`, `account/mod.rs`, `settings-dialog.tsx`), no en los planes.

## Lo que hace cada una de verdad

| # | Hoy | Dónde | Quién la puede usar | A quién afecta | Qué se borra y dónde | ¿Se deshace? |
|---|---|---|---|---|---|---|
| 1 | «Olvidar dispositivo» | «Compartir y dispositivos», menú del aparato | Cualquiera, en su propio aparato | Solo a este aparato: borra su registro del otro | Solo la conexión de emparejado (`library_peers`, avisos y servidores conocidos). **Ningún libro.** El otro aparato conserva todo | Sí: se empareja otra vez |
| 2 | «Quitar un dispositivo» | «Personas y permisos», aparato de una persona | Quien administra la biblioteca | A ese aparato, en **todas** las bibliotecas donde escribe, y a su dueña solo en que ese aparato deja de contar | Se retira el acceso del aparato y deja de ser de su persona; lo que envíe queda como petición. Cuando se conecte, **borra** esas bibliotecas | No hay botón para deshacerlo (por mirar si se puede volver a dar acceso nuevo al mismo aparato) |
| 3 | «Quitar» | Cuenta › «Mis dispositivos» | La propia cuenta, desde otro aparato | A ese aparato: sale de la cuenta | Cuando se conecte, **borra** sus bibliotecas y deja de estar conectado a la cuenta | Sí, con «Deshacer», **hasta que ese aparato se conecte** |
| 4 | «Dejar este dispositivo a otra persona» | «Ajustes» | Cualquiera, en su propio aparato | Solo a este aparato | **Al momento** borra de aquí todas las bibliotecas y la cuenta abierta; queda como recién instalado. No toca la nube ni otros aparatos | No (guarda antes una copia de seguridad) |

La confusión viene de que 1 y 2 usan casi la misma palabra y hacen cosas contrarias (1 no quita acceso a nada; 2 sí), y de que 2 y 3 se parecen (los dos acaban borrando el aparato al conectarse) pero uno lo decide quien administra la biblioteca y el otro la dueña de la cuenta.

## Propuesta

Una idea para cada una: **«Desemparejar»** es solo la wifi de casa; **«Retirar el acceso»** es lo de quien administra una biblioteca; **«Sacar de mi cuenta»** es la cuenta; y **«Dejar como nuevo»** es el aparato entero.

| Hoy | Qué hace | Nombre propuesto | Frase de ayuda | Confirmación (título · texto · botón) |
|---|---|---|---|---|
| «Olvidar dispositivo» | Deja de emparejar con este aparato; no borra nada | **Desemparejar** | «Dejáis de pasaros cambios. No se borra nada.» | «¿Desemparejar {{name}}?» · «Dejaréis de pasaros cambios. No se borra ningún libro ni se le quita el acceso a nada. Para volver, habrá que emparejarlos otra vez.» · «Desemparejar» |
| «Quitar un dispositivo» (Personas) | Retira el acceso del aparato a las bibliotecas; las borrará al conectarse | **Retirar el acceso de este aparato** | «Deja de recibir esta biblioteca y la borrará cuando se conecte. Su dueña conserva sus demás aparatos.» | «¿Retirar el acceso de {{device}}?» · el texto que ya hay (qué bibliotecas dejará de recibir; no recibirá más cambios y lo que envíe no se tendrá en cuenta; se borrarán de él al conectarse) · «Retirar el acceso» |
| «Quitar» (Mis dispositivos) | Saca el aparato de tu cuenta; borrará sus bibliotecas al conectarse | **Sacar de mi cuenta** | «Este aparato deja de usar tu cuenta y borrará sus bibliotecas cuando se conecte. Puedes deshacerlo hasta entonces.» | «¿Sacar {{device}} de tu cuenta?» · el texto que ya hay (saldrá de tu cuenta; las bibliotecas que solo estén ahí se perderán; hasta que se conecte puedes deshacerlo) · «Sacar de mi cuenta» |
| «Dejar este dispositivo a otra persona» | Borra todo de este aparato y cierra la cuenta, al momento | **Dejar este móvil como nuevo** | «Borra de aquí todas las bibliotecas y tu cuenta, para dárselo a otra persona. Lo que hay en la nube y en tus otros aparatos no se toca.» | «¿Dejar este móvil como nuevo?» · «Se borran de aquí todas las bibliotecas y tu cuenta abierta. No se puede deshacer: guarda antes una copia de seguridad si hace falta.» · «Dejar como nuevo» |

Notas para decidir:
- «Dispositivo» sigue en los textos de la app, donde ya está en todas partes; la propuesta cambia el **verbo** y no la palabra «aparato/dispositivo» (la guía usa «aparato» en el texto corrido).
- «Retirar» ya se usa en Peticiones («Retirar» una petición, que en inglés es «Cancel»). Si molesta, la alternativa es «Quitar el acceso».
- Cada nombre cambia a la vez en `es.json` y en `en.json` y en la Ayuda («lost»); la guía (`remove-someone.md`, `lost-phone.md`) se ajusta después.
- Si el maintainer prefiere no tocar la app, bastan las frases de ayuda: lo que más confunde es 1 frente a 2.
