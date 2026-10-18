---
title: Conectar la app a tu servidor
description: Cómo apuntar la app de Kosmotheka a tu propio servidor, qué direcciones acepta, qué pasa si no responde y cómo se usan las invitaciones.
sidebar:
  order: 5
---

La app oficial se conecta a tu servidor sin compilar nada. Solo necesitas la dirección pública (la misma que pusiste en `DEFAULT_URI`) y que `GET /v1/server` responda.

## Paso a paso

1. En la app, abre **Ajustes › Cuenta y nube** y pulsa **Iniciar sesión o crear cuenta**.
2. En la pantalla de iniciar sesión verás «En *servidor oficial*» y, al lado, **Otro servidor**. Púlsalo.
3. Escribe la dirección completa en el campo **Servidor**, con `https://` y sin ruta: `https://kosmotheka.tu-dominio.example`. Una barra final se ignora.
4. Al salir del campo, la app llama a `/v1/server`. Si el servidor responde, aparece un punto verde con «Responde · solo con email y contraseña» (el texto es fijo: si el servidor ofrece Google, el botón aparece igualmente). Si no, ves el error debajo del campo (más abajo, en [cuando el servidor no responde](#cuando-el-servidor-no-responde)).
5. Inicia sesión con una cuenta que ya exista allí, o pulsa **Crear cuenta**.

**Usar el oficial** (arriba a la derecha del campo) vuelve al servidor oficial.

### Al crear la cuenta

Una cuenta nueva se hace en dos pasos:

1. Nombre, correo y contraseña (8 caracteres como mínimo). El servidor responde siempre lo mismo, exista ya la cuenta o no.
2. Un código de 6 cifras que el servidor manda por correo y que caduca a los 15 minutos. Hasta que se confirma, la cuenta no sirve; **las cuentas sin confirmar se borran a las 24 horas**.

Por eso necesitas [el correo configurado](/es/run-your-server/configuration/#el-correo): sin él, nadie recibe el código. Si el servidor tiene lista de correos permitidos (ver [quién puede registrarse](/es/run-your-server/administration/#quién-puede-registrarse)), la app avisa de que «este servidor solo admite algunas direcciones». Quien no está en la lista llega igualmente al paso del código, pero **no le llega nada**.

Al iniciar sesión, el dispositivo se da de alta en el servidor con su nombre, un identificador y su clave pública, y firma la entrada para **esa dirección**: una firma hecha para otro servidor no vale en el tuyo. Si más tarde cambias la dirección pública del servidor (`DEFAULT_URI`), **nadie podrá iniciar sesión** hasta que la app tenga la nueva.

### El aviso de «no oficial»

La app de Kosmotheka es la misma para el servidor oficial (`api.kosmotheka.app`) y para el tuyo, y funciona con cualquier servidor, pero **quien la usa tiene que saber cuándo sus datos están en un servidor que no es el oficial**. Por eso, con el tuyo, las personas verán «Esta nube no es la oficial de Kosmotheka: la lleva *tu-dominio*» al escribir su dirección, en su cuenta y en «Compartir y dispositivos», al abrir una invitación tuya y en las bibliotecas que vivan en él. Es un aviso, no un bloqueo: no hace falta configurar nada en el servidor, y no cambia al poner otro nombre. «Oficial» es exactamente `https://api.kosmotheka.app`: un servidor de pruebas en tu red también sale como no oficial.

### Cambiar de servidor

La app solo deja cambiar de servidor **sin sesión iniciada**. Quien ya está conectado a otro tiene que cerrar sesión antes (**Ajustes › Cuenta y nube › Cerrar sesión**: no borra nada del dispositivo).

## Qué direcciones acepta

La app comprueba la dirección antes de llamar a nada:

- **`https`**, siempre. Es lo que debes usar.
- **`http`** solo si la dirección es de la red de casa o de la propia máquina: `localhost`, una IP privada (`10.x.x.x`, `172.16.x.x` a `172.31.x.x`, `192.168.x.x`), la de bucle local de IPv6 (`[::1]`) o `10.0.2.2` (el emulador de Android). Con un nombre como `servidor.local` o una IP de una VPN (por ejemplo `100.x.x.x`), `http` se rechaza.
- Sin usuario ni contraseña en la dirección, sin ruta (`/algo`) y sin parámetros.

Una dirección que no cumple da «Esta dirección no es válida. Comprueba que sea la de un servidor Kosmotheka.» El `http` es para probar en casa: la contraseña y las bibliotecas viajarían sin cifrar.

## Cuando el servidor no responde

Si la app no puede hablar con el servidor, dice **«El servidor no responde. Puede estar en mantenimiento, o la dirección no es correcta»** y ofrece **Reintentar**. Qué mirar, por orden:

1. Abre `https://tu-dominio/v1/server` en un navegador. Debe devolver un JSON con lo que ofrece tu servidor.
2. Si no abre, es nginx, PHP-FPM o el certificado: la guía de [problemas comunes](/es/run-your-server/troubleshooting/) tiene los síntomas habituales.
3. Si abre en el navegador pero la app falla, comprueba que la dirección escrita en la app es la misma que la pública y que el reloj del dispositivo es correcto (un certificado se rechaza si la fecha está lejos de la real).

## Las invitaciones de un servidor propio

Quien administra una biblioteca puede invitar a otra persona **por correo** o con un **enlace** para copiar. El enlace tiene la forma `<INVITATION_BASE_URL>/i/<código>`, vale 24 horas y una sola vez.

**En un servidor propio, el enlace no se abre con un toque.** Los enlaces que abren la app directamente (en Android) se verifican en el momento de compilar la app, y solo están verificados los del dominio oficial. Los de tu servidor se **copian y se pegan en la app**:

1. Quien recibe el enlace lo copia (del mensaje o del correo; no hace falta abrirlo).
2. En la app, **Tengo una invitación** (en el diálogo de iniciar sesión, en **Ajustes** o en la pantalla de la cuenta): pega el enlace y pulsa **Ver**.
3. La app enseña a qué biblioteca la han invitado y con qué cuenta entrará. Hasta pulsar **Entrar** no pasa nada.

Cosas que conviene saber:

- La app **solo acepta enlaces del servidor al que está conectada**, y solo si empiezan por la dirección que el servidor publica como `invitation_base`. Un enlace de otro sitio se rechaza con «Este enlace es de otro servidor distinto al de tu cuenta».
- Quien invita copia el enlace una sola vez, al crearlo: si cierra la ventana sin copiarlo, tiene que anular la invitación y crear otra.
- El correo de la invitación lleva el enlace, pero **tu servidor no sirve ninguna página en `/i/<código>`**: abrirlo en un navegador da un 404 de tu nginx. Funciona porque la app lee el código del texto, no porque lo abra. Dilo a quien invites: «copia el enlace y pégalo en la app».
- Para `INVITATION_BASE_URL` usa la dirección de tu servidor (`https://tu-dominio`). Una dirección del dominio oficial se descarta.
