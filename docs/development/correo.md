# Correo del proyecto: ImprovMX, Gmail y Brevo

> Decisión: [D80](../DECISIONS.md). Tres piezas:
> - **ImprovMX** (gratis) recibe el correo de `kosmotheka.com` y lo reenvía al buzón.
> - **Gmail** (el buzón del proyecto) es el buzón: allí se lee y se responde.
> - **Brevo** envía: las respuestas como `info@` (con «Enviar como» de Gmail) y los correos del servidor (códigos de registro y de recuperación, D49 y D50).
>
> El DNS sigue en el proveedor del VPS. Ninguna clave va al repositorio: van en `.env.local` del servidor y en el gestor de contraseñas.

## Direcciones

| Dirección | Qué es | Para qué |
|---|---|---|
| `info@kosmotheka.com` | Alias de ImprovMX → Gmail | La dirección del proyecto: altas en servicios externos, contacto de la web y de Google Play |
| `privacy@kosmotheka.com` | Alias de ImprovMX → Gmail | Contacto que pide la política de privacidad (RGPD) |
| `dmarc@kosmotheka.com` | Alias de ImprovMX → Gmail | Informes DMARC, con un filtro para que no llenen la bandeja |
| `no-reply@kosmotheka.com` | Remitente de Brevo, no es buzón | Correos automáticos del servidor |
| el buzón de Gmail del proyecto | El buzón | **No se da a nadie** ni se usa para registrarse en servicios, salvo en ImprovMX |

**Regla:** todos los servicios se dan de alta con `info@kosmotheka.com`. Así, cambiar el buzón (D80 prevé hacerlo en enero de 2027) solo cambia los MX.

**Excepción: ImprovMX se da de alta con el buzón de Gmail del proyecto.** Es quien hace funcionar `info@`: si falla, a `info@` tampoco llegaría el correo para recuperar su cuenta.

## 0. Gmail

1. Crear el buzón de Gmail del proyecto.
2. Activar el segundo factor, guardar los códigos de recuperación en el gestor de contraseñas y poner el correo personal como recuperación.

## 1. ImprovMX: recibir

1. Alta en [improvmx.com](https://improvmx.com) con el buzón de Gmail del proyecto (la excepción a la regla), el dominio `kosmotheka.com` y el Gmail como destino. Activar el segundo factor.
2. DNS en el proveedor del VPS:
   - Borrar cualquier MX que haya.
   - **MX** `@` → `mx1.improvmx.com.`, prioridad 10.
   - **MX** `@` → `mx2.improvmx.com.`, prioridad 20.
   - **SPF**: ver [DNS común](#4-dns-común).
3. Crear los alias `info`, `privacy` y `dmarc`, todos hacia el buzón de Gmail del proyecto. **Sin alias comodín (`*`)**: solo atrae spam.
4. Esperar a que el panel marque el dominio como correcto y probar a escribir a `info@` desde otra cuenta.

El plan gratuito permite 1 dominio, 25 alias y 500 reenvíos al día. No incluye SMTP: para enviar está Brevo.

## 2. Brevo: enviar

1. Alta con **`info@kosmotheka.com`** (el correo de confirmación llega a Gmail a través de ImprovMX). Activar el segundo factor y guardar los códigos.
2. *Senders, Domains & Dedicated IPs → Domains*: añadir `kosmotheka.com` y **autenticarlo** con los registros que muestre el panel: el TXT `brevo-code`, los CNAME de DKIM y el DMARC (si ya está creado, se mantiene el nuestro).
3. *Senders*: añadir `info@kosmotheka.com` («Kosmotheka») y `no-reply@kosmotheka.com` («Kosmotheka»).
4. *SMTP & API → SMTP*: apuntar el servidor (`smtp-relay.brevo.com`, puerto 587) y el usuario, y generar **una clave SMTP para Gmail** llamada `gmail-send-as`. La del servidor se genera aparte cuando llegue el hito 7.1, para poder revocar una sin romper la otra.

El plan gratuito permite unos 300 correos al día entre todo.

## 3. Gmail: responder como `info@`

1. Gmail → *Configuración → Cuentas e importación → Enviar como → Añadir otra dirección de correo*.
2. Nombre «Kosmotheka», dirección `info@kosmotheka.com`, con «Tratar como alias» marcado.
3. Servidor SMTP `smtp-relay.brevo.com`, puerto 587, el usuario SMTP de Brevo y la clave `gmail-send-as`, con TLS.
4. Gmail envía un código a `info@`, que llega a la misma bandeja a través de ImprovMX: confirmarlo.
5. En *Enviar como*: marcar `info@` como dirección predeterminada y «Responder desde la misma dirección a la que se envió el mensaje».
6. Un filtro para los correos a `dmarc@`: saltar la bandeja de entrada y aplicar la etiqueta «DMARC».

**Caduca en enero de 2027**, cuando Gmail retira «Enviar como» para direcciones ajenas. Entonces hay que decidir el buzón definitivo (D80).

## 4. DNS común

Registros de `kosmotheka.com` que afectan a más de un servicio:

| Tipo | Nombre | Valor |
|---|---|---|
| TXT | `@` | `v=spf1 include:spf.improvmx.com include:spf.brevo.com ~all` |
| TXT | `_dmarc` | `v=DMARC1; p=none; rua=mailto:dmarc@kosmotheka.com` |

- **Un solo registro SPF.** Si hay dos, los dos dejan de valer. Si los paneles de ImprovMX o Brevo piden otro `include`, se añade a este mismo registro.
- **DMARC** empieza con `p=none`, para recibir informes sin rechazar nada. Cuando los informes muestren unas semanas de correos que pasan SPF o DKIM alineado, se endurece a `p=quarantine`.
- Para comprobarlo, enviar un correo a [mail-tester.com](https://www.mail-tester.com) desde Gmail como `info@` y, cuando exista, otro desde el servidor.

## 5. El servidor (hito 7.1)

`symfony/mailer` con el bridge `symfony/brevo-mailer`, y en `server/.env.local`:

```
MAILER_DSN=brevo+smtp://USUARIO:CLAVE@default
```

o con una API key de Brevo: `MAILER_DSN=brevo+api://CLAVE@default`. Si hiciera falta cambiar de proveedor, basta con cambiar el DSN (Resend, Mailjet, Amazon SES…) y su DNS; recibir no se ve afectado.

## Servidores propios

Quien monte su propio servidor no necesita nada de esto: configura en `MAILER_DSN` el proveedor que quiera, cualquiera que admita `symfony/mailer`. La guía de autoalojamiento (hito 7.9) lo explicará.
