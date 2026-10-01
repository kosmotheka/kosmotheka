# Servicios externos que hay que configurar

> Lista de cuentas, proyectos y claves fuera del repositorio que necesitan la nube y la publicación. Cada entrada se convierte en una guía paso a paso cuando llega su hito; hasta entonces, solo qué es, para qué sirve y de qué decisión sale. **Ninguna clave ni secreto va en el repositorio** (AGENTS.md): van en `.env.local` del servidor o en los secretos de GitHub, con copia fuera.

| Servicio | Para qué | Cuándo | Decisión | Estado |
|---|---|---|---|---|
| **Correo con el dominio propio** (por decidir: servicio gratuito) | Dar de alta los demás servicios con una dirección del proyecto, y el contacto de la web y de Google Play | Antes del hito 7.1 | — | Pendiente de elegir |
| **Cuenta en X** con el nombre del proyecto | Reservar el nombre; no para iniciar sesión | Cuando haya correo propio | D70 | Pendiente |
| **Google Cloud: proyecto, pantalla de consentimiento OAuth y credenciales** | «Continuar con Google» | Hito 7.1 | D77 | Pendiente |
| **Proveedor de correo** (por decidir) y DNS del dominio (SPF, DKIM, DMARC) | Códigos de registro y de recuperación | Hito 7.1 | D49 | Pendiente de elegir |
| **Dominios** (D79): `api.kosmotheka.app`, `kosmotheka.app` con su página de instalación y `assetlinks.json` (sin la redirección actual a `.com`), y sus certificados HTTPS | Que la app hable con la nube, las invitaciones y volver de Google a la app | Hito 7.1 (`assetlinks.json`) y 7.9 | D48, D79 | Pendiente |
| **Claves de proveedores:** BoardGameGeek, TMDB, IGDB o RAWG, Comic Vine | Búsquedas con clave a través de la nube | Hito 7.8 | D62 | Pendiente |
| **Firebase: proyecto, FCM y cuenta de servicio del servidor** | Notificaciones con la app cerrada en Android | Fase 8 | D75 | Pendiente |
| **Google Play: cuenta de servicio para la API** y clave de subida | Subir el `.aab` desde el CI | Fase 8 | D74 | Cuenta de desarrollador aprobada; lo demás, pendiente |

## Notas para cuando llegue cada uno

- **Google Cloud y Firebase pueden ser el mismo proyecto:** un proyecto de Firebase es un proyecto de Google Cloud. Conviene crearlo una vez, con la cuenta del proyecto, y usarlo para OAuth (D77), FCM (D75) y, si se quiere, la cuenta de servicio de Google Play (D74).
- **OAuth de Google:** con los permisos básicos (email y perfil) la verificación de Google es ligera, pero pide la página de la política de privacidad y la del proyecto publicadas en el dominio. Se cruza con la landing (Fase 8).
- **Servidores propios:** quien monte el suyo no necesita nada de esta lista para empezar; Google, el correo de un proveedor y las claves son opcionales y se configuran con variables de entorno. La guía de autoalojamiento (hito 7.9) lo explicará.
