---
title: Antes de empezar
description: Lo que tienes que saber antes de montar tu propio servidor de Kosmotheka, y qué hace falta para hacerlo.
sidebar:
  order: 1
---

El servidor es opcional: la app funciona entera sin él, con su base de datos en tu móvil o tu ordenador. Un servidor sirve para tener tus bibliotecas en todos tus dispositivos sin compartir wifi, y para compartirlas con otras personas por invitación. Esta guía es para quien quiere alojar el suyo en vez de usar el oficial.

## Lo que tienes que saber

Estas son las limitaciones de montar tu propio servidor. Están aquí, al principio, para que decidas con ellas delante.

- **La app oficial puede conectarse a tu servidor.** En «Otro servidor» eliges la dirección del tuyo. No hace falta compilar nada.
- **Sin avisos con la app cerrada.** La app se pone al día al abrirla. El servidor puede avisar al momento a las apps que están abiertas (con un componente opcional, Mercure), pero no manda notificaciones al móvil.
- **Los enlaces de invitación se pegan en la app.** Los de un servidor propio no se abren solos con un toque: quien recibe el enlace lo copia y lo pega en la app.
- **Entrar con Google, el correo y las claves de los buscadores son opcionales y se configuran aparte.** Sin correo, nadie puede confirmar una cuenta nueva con un código (ver [la configuración](/es/run-your-server/configuration/#el-correo)). Cada buscador (películas, videojuegos, cómics…) necesita una clave que pides tú a su proveedor, con sus condiciones.
- **Quien administra el servidor puede leer lo que se sube a él.** Las bibliotecas no se cifran de extremo a extremo: están en la base de datos del servidor. Si alojas el servidor de otras personas, díselo.
- **El nombre y el logo no son tuyos.** El código es libre, pero «Kosmotheka» y su logo no se pueden usar para hacer pasar tu servidor o tu app por el oficial (ver [TRADEMARKS.md](https://github.com/kosmotheka/kosmotheka/blob/main/TRADEMARKS.md)). Que tu servidor «funcione con Kosmotheka» sí se puede decir.

## Qué hace falta

- **Un dominio con HTTPS.** La app de Android de las tiendas no se conecta a un servidor sin HTTPS, y de todos modos las contraseñas y las bibliotecas no deben viajar sin cifrar. Necesitas un nombre (por ejemplo `kosmotheka.tu-dominio.example`) apuntando a la máquina, y un certificado (la guía usa Let's Encrypt).
- **Un servidor Linux con recursos modestos.** Para una familia basta una máquina pequeña: 1 CPU, 2 GB de memoria y unos pocos GB de disco. Para hacerte una idea: en las pruebas de carga, subir una biblioteca de 2.000 libros de una vez lleva alrededor de un minuto, un cambio normal llega en menos de una décima de segundo, y una petición normal gasta unos 120 MB de memoria de PHP. Si la máquina comparte sitio con otros servicios, deja a PHP un `memory_limit` acotado (la guía lo hace).
- **Saber administrar un servidor Linux:** instalar paquetes, editar la configuración de nginx, usar systemd y leer un registro de errores. No hace falta conocer Symfony ni PHP.

## Los dos caminos

El servidor se puede montar de dos maneras con las mismas piezas (PHP, PostgreSQL, nginx y un proceso de tareas programadas):

1. **[Sin Docker](/es/run-your-server/without-docker/)**, instalando todo en la máquina. Es como funciona el servidor oficial y está probado.
2. **Con Docker.** Todavía no está escrito. Hoy los ficheros de Docker del repositorio son solo para desarrollo (corren como administrador y llevan un depurador), y no sirven para un servidor real. Cuando haya una imagen de producción, esta guía tendrá su página.

Después de montarlo:

- [La configuración, variable a variable](/es/run-your-server/configuration/).
- [Copias de seguridad y cómo restaurar](/es/run-your-server/backups/).
- [Actualizar a una versión nueva](/es/run-your-server/updating/).
- [Problemas comunes](/es/run-your-server/troubleshooting/).
