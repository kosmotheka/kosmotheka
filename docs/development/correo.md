# Correo del proyecto: Zoho Mail y Brevo

> Decisión: [D80](../DECISIONS.md). Dos piezas independientes:
> - **Zoho Mail** (plan Forever Free): el buzón de `kosmotheka.com`, donde recibimos y desde donde respondemos, con su web y su app.
> - **Brevo**: el correo que envía el servidor (códigos de registro y de recuperación, D49 y D50).
>
> El DNS sigue en el proveedor del VPS. Ninguna clave va al repositorio: van en `.env.local` del servidor y en el gestor de contraseñas.

## Direcciones

| Dirección | Dónde | Para qué |
|---|---|---|
| `info@kosmotheka.com` | Zoho, usuario principal | Altas en servicios externos, contacto de la web y de Google Play |
| `privacidad@kosmotheka.com` | Zoho, alias de `info@` | Contacto que pide la política de privacidad (RGPD) |
| `no-reply@kosmotheka.com` | Brevo, remitente | Correos automáticos del servidor; no es un buzón |

Los nombres son una propuesta; lo que importa es que el buzón sea uno solo y el resto, alias.

## Orden

1. Zoho primero: así existe la dirección del proyecto.
2. Brevo después, dado de alta ya con `info@kosmotheka.com`.
3. Los demás servicios (Google Cloud, Firebase, X…), con `info@kosmotheka.com`.

## 1. Zoho Mail

1. Alta en el plan **Forever Free**, eligiendo el **centro de datos europeo** (zoho.eu). Si en la página de precios no aparece el plan gratuito, no seguir: el plan B es Zoho Mail Lite (D80).
2. **Cuenta nueva, no la de rpgutilities.** El plan gratuito admite un solo dominio por organización, y el email de contacto que pide el alta no puede tener ya una cuenta de Zoho. Hay que usar otro email personal.
3. Añadir el dominio `kosmotheka.com` y verificarlo con el registro TXT que da Zoho.
4. Crear el usuario `info@kosmotheka.com`. Esa es la dirección con la que se entra a partir de ahí.
5. DNS, con los valores exactos que muestre el panel de Zoho:
   - **MX**: los tres de Zoho, con sus prioridades (en el centro de datos EU, `mx.zoho.eu`, `mx2.zoho.eu` y `mx3.zoho.eu`). Quitar cualquier otro MX.
   - **SPF**: ver el apartado [DNS común](#3-dns-común); hay uno solo para Zoho y Brevo.
   - **DKIM**: el TXT que genera Zoho (*Email Authentication → DKIM*).
6. **Seguridad de la cuenta:** activar el segundo factor, guardar los códigos de recuperación en el gestor de contraseñas y poner el **correo personal** como email de recuperación. Así recuperar el buzón no depende del propio buzón.
7. Crear el alias `privacidad@` y probar a enviar y recibir desde una cuenta externa.
8. Instalar la app **Zoho Mail** en el móvil.

## 2. Brevo

1. Alta con `info@kosmotheka.com`. Activar el segundo factor y guardar los códigos.
2. *Senders, Domains & Dedicated IPs → Domains*: añadir `kosmotheka.com` y **autenticarlo** con los registros que muestre el panel: el TXT `brevo-code`, los dos CNAME de DKIM y el DMARC (si ya existe, se mantiene el nuestro).
3. Añadir el remitente `no-reply@kosmotheka.com` («Kosmotheka»).
4. Generar las credenciales para el servidor. Se puede elegir entre:
   - **API key** (*SMTP & API → API Keys*), con el bridge `symfony/brevo-mailer`: `MAILER_DSN=brevo+api://CLAVE@default`
   - o **SMTP** (*SMTP & API → SMTP*): `MAILER_DSN=brevo+smtp://USUARIO:CLAVE@default`
5. El `MAILER_DSN` va en `server/.env.local` y en el gestor de contraseñas, nunca en el repositorio.

El plan gratuito permite unos 300 correos al día, de sobra para registros y recuperaciones. Si hiciera falta cambiar de proveedor, basta con cambiar el `MAILER_DSN` (Resend, Mailjet, Amazon SES…) y su DNS; el buzón de Zoho no se toca.

## 3. DNS común

Registros de `kosmotheka.com` que afectan a los dos servicios:

| Tipo | Nombre | Valor |
|---|---|---|
| TXT | `@` | `v=spf1 include:zohomail.eu include:spf.brevo.com ~all` |
| TXT | `_dmarc` | `v=DMARC1; p=none; rua=mailto:info@kosmotheka.com` |

- **Un solo registro SPF.** Si hay dos, los dos dejan de valer. Los `include` exactos son los que indiquen los paneles de Zoho y Brevo.
- **DMARC** empieza con `p=none` para recibir informes sin rechazar nada. Cuando los informes muestren que todo pasa SPF y DKIM durante unas semanas, se endurece a `p=quarantine`.
- Para comprobarlo, enviar un correo a [mail-tester.com](https://www.mail-tester.com) desde Zoho y otro desde el servidor.

## Servidores propios

Quien monte su propio servidor no necesita nada de esto: configura en `MAILER_DSN` el proveedor que quiera, cualquiera que admita `symfony/mailer`. La guía de autoalojamiento (hito 7.9) lo explicará.
