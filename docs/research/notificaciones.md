# Notificaciones con la app cerrada (8.3, D75)

> Primera nota, 16/10/2026. Sin probar nada: es lo que dicen los repositorios y la documentación de Google. Sirve para preparar el encargo de 8.3.

## Cómo funciona, en llano

1. La app pide a Google un **token**: una dirección para ese móvil, sin datos de la persona.
2. La app le da el token a nuestro servidor, que lo guarda junto al dispositivo.
3. Cuando pasa algo (una petición nueva…), el servidor manda a Google «avisa a este token: *Tienes una petición nueva*». Google lo entrega aunque la app esté cerrada.
4. Al tocar el aviso, se abre la app, que sincroniza por el camino de siempre.

Google ve el token, la hora y el texto del aviso. Por eso el texto es genérico (D75) y nunca lleva nada de la biblioteca.

## Plugins de Tauri 2

Tauri no tiene un plugin oficial de push; su plugin `notification` solo hace avisos locales. Los de la comunidad:

| Plugin | Licencia | Madurez | Notas |
|---|---|---|---|
| [tauri-plugin-notifications](https://github.com/Choochmeque/tauri-plugin-notifications) (Choochmeque) | MIT | El más activo (85 estrellas, más de 600 commits) | Avisos locales y push en una sola pieza; FCM con la *feature* `push-notifications`; en Linux, UnifiedPush. `registerForPushNotifications()` da el token; `onAction()` para el toque |
| [tauri-plugin-mobile-push](https://github.com/yanqianglu/tauri-plugin-mobile-push) | MIT o Apache-2.0 | v0.1, 10 estrellas | Solo móvil; en escritorio no hace nada. `onNotificationTapped()` |
| [tauri-plugin-remote-push](https://crates.io/crates/tauri-plugin-remote-push) | Por mirar | Por mirar | Solo FCM y APNs |

**Recomendación:** empezar por `tauri-plugin-notifications` y probarlo en un móvil antes de escribir nada más. Si no convence, escribir el nuestro con poco Kotlin: el SDK de Firebase hace casi todo.

## Lo que pide Firebase

- **Un proyecto de Firebase** a nombre del maintainer, con la app Android `app.kosmotheka` dada de alta.
- **`google-services.json`** en `gen/android/app/` y el plugin de Gradle de Google Services. Ese fichero identifica el proyecto y no es un secreto: puede ir en el repositorio público, pero conviene restringir su clave de API en la consola de Google Cloud.
- **En el servidor, una cuenta de servicio** de ese proyecto para enviar con la API HTTP v1 de FCM. **Esa sí es un secreto**: va en el servidor, nunca en el repositorio.
- Una dependencia nueva en el APK (Firebase Messaging): su aviso y sus términos van en «Licencias», como ML Kit.

## Lo que esto cambia para quien monta su servidor

**Solo puede mandar avisos a la app oficial quien tenga la cuenta de servicio de nuestro proyecto de Firebase.** Un servidor propio no la tiene, así que no puede enviar notificaciones a la app de Google Play. Hay dos caminos:

| Camino | A favor | En contra |
|---|---|---|
| **Sin avisos con la app cerrada en servidores propios** | Nada que montar; D75 ya dice que el push es opcional y que, sin él, la app sincroniza al abrirse | Quien monta su servidor no tiene avisos con la app cerrada |
| Un relé en el servidor oficial que reenvía los avisos de los demás | Avisos para todos | Nuestro servidor vería quién avisa a quién y cuándo; más superficie y más trabajo |

**Recomendación:** el primero, dicho en la guía de autoalojamiento. El relé solo si alguien lo pide.

## Para la prueba cerrada: borrar la cuenta desde la web

Toda app que permite crear una cuenta tiene que ofrecer en Google Play dos caminos para borrarla ([Play Console Help](https://support.google.com/googleplay/android-developer/answer/13327111)):

- **un camino dentro de la app**, que ya existe;
- **un enlace web** donde pedir el borrado. Vale una página que explique cómo hacerlo y dé un correo: `info@kosmotheka.com` (D80).

La página tiene que nombrar la app como en la ficha, cargar sin errores y tener el borrado a la vista. También tiene que decir qué se guarda tras borrar y por qué; el registro de actividad, un año (D66). Es una página más en la landing (8.1).
