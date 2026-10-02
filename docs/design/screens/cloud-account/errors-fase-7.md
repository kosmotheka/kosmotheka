# Cuenta y nube (Fase 7) — propuesta para docs/design/errors.md

> Mismo patrón que el resto del catálogo. Niveles: **info** · **aviso** (ámbar, no se perdió nada) · **error** (rojo, la acción no se hizo). Los botones van en negrita.

| Código | Nivel | Español | English |
|---|---|---|---|
| `code_incorrect` | error | Código incorrecto. Usa el del último correo que te ha llegado. | Incorrect code. Use the one in the latest email you received. |
| `code_expired` | error | Este código ha caducado. Los códigos valen 15 minutos. Pide otro y úsalo en cuanto llegue. **Botón:** «Enviar otro código». | This code has expired. Codes last 15 minutes. Ask for a new one and use it as soon as it arrives. **Button:** “Send another code”. |
| `code_attempts` | error | Demasiados intentos con este código. Ya no sirve. Pide otro: llega en un momento. **Botón:** «Pedir otro código». | Too many attempts with this code. It no longer works. Ask for a new one; it arrives in a moment. **Button:** “Get a new code”. |
| `rate_limited` | aviso | Demasiadas peticiones. Espera {n} minutos e inténtalo de nuevo. | Too many requests. Wait {n} minutes and try again. |
| `bad_credentials` | error | El email o la contraseña no son correctos. Revisa cómo está escrito el email. Si no recuerdas la contraseña, pon una nueva. **Botón:** «Recuperar la contraseña». | The email or password is incorrect. Check how the email is spelled. If you don’t remember your password, set a new one. **Button:** “Reset password”. |
| `email_unconfirmed` | aviso | Falta confirmar tu email. Al crear la cuenta te enviamos un código. Si no lo encuentras, pide otro. **Botón:** «Enviar otro código». | Your email isn’t confirmed yet. We sent you a code when you created the account. If you can’t find it, ask for another. **Button:** “Send another code”. |
| `account_locked` | error | Esta cuenta está bloqueada. Si crees que es un error, escribe a quien gestiona el servidor. | This account is locked. If you think this is a mistake, contact whoever runs the server. |
| `totp_incorrect` | error | El código no es correcto. Usa el que se ve ahora en la app: cambia cada 30 segundos. Comprueba que la hora del móvil sea automática. **Botón:** «Usar un código de respaldo». | That code isn’t right. Use the one showing now in the app; it changes every 30 seconds. Make sure your phone sets the time automatically. **Button:** “Use a backup code”. |
| `backup_incorrect` | error | Ese código de respaldo no vale. Puede que ya lo usaras: cada uno vale una vez. Prueba con otro de la lista. | That backup code doesn’t work. You may have used it already; each one works once. Try another from your list. |
| `session_revoked` | info | Se ha cerrado la sesión. La contraseña se cambió en otro dispositivo. No se pierde nada: los cambios sin subir se enviarán al volver a entrar. **Botón:** «Iniciar sesión». | You’ve been signed out. Your password was changed on another device. Nothing is lost: unsent changes will upload when you sign back in. **Button:** “Sign in”. |
| `device_removed` | info | Este dispositivo se quitó de una cuenta. Se quitó desde otro dispositivo y sus bibliotecas se han borrado. La app está lista para usar, con o sin cuenta. | This device was removed from an account. It was removed from another device and its libraries were deleted. The app is ready to use, with or without an account. |
| `cloud_offline` | aviso | Sin conexión con la nube. Todo sigue funcionando aquí y por la red de casa; los cambios se subirán cuando vuelva la conexión. | Not connected to the cloud. Everything still works here and on your home network; changes will upload once you’re back online. |
| `server_unavailable` | aviso | El servidor no responde. Puede estar en mantenimiento, o la dirección no es correcta. Inténtalo en unos minutos. **Botón:** «Reintentar». | The server isn’t responding. It may be under maintenance, or the address is wrong. Try again in a few minutes. **Button:** “Retry”. |
| `invite_invalid` | error | Esta invitación ya no vale. Ya se usó o han pasado más de 24 horas. Pide a {nombre} que te mande otra. | This invitation is no longer valid. It was already used or is over 24 hours old. Ask {name} to send you a new one. |
| `password_short` | error | La contraseña necesita al menos 8 caracteres.  | Your password needs at least 8 characters.  |

## Dónde se ven

- Inicio de sesión y códigos: en un bloque bajo los campos; el formulario no se cierra.
- `session_revoked` y `device_removed`: pantalla propia al abrir la app.
- `cloud_offline`: indicador discreto (barra superior en escritorio, cabecera de Compartir en móvil) y fila de la nube; nunca bloquea.
- `invite_invalid`: pantalla propia al abrir el enlace.

> El servidor no dice cuántos intentos quedan ni si un email ya tiene cuenta (para no revelar qué emails están registrados). Crear cuenta siempre pasa a la pantalla del código; si el email ya tenía cuenta, le llega un correo avisando.
