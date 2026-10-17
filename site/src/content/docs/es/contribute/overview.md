---
title: Las piezas y cómo encajan
description: De qué partes está hecho Kosmotheka y qué pasa, paso a paso, cuando se añade un libro.
sidebar:
  order: 1
---

Kosmotheka es un catálogo compartido para lo que tiene una casa: libros primero, y después cómics, juegos de mesa, películas, videojuegos y música. Esta página es el mapa. Si quieres tocar el código, empieza aquí.

Hay tres ideas que lo explican casi todo:

- **Local primero.** La app siempre lee y escribe su propia base de datos SQLite y funciona entera sin servidor ni cuenta. *Por qué:* un catálogo de casa tiene que abrirse en la tienda, sin cobertura.
- **Servidor opcional, y con autoridad cuando existe.** Quien quiera, conecta la app a un servidor. En ese modo el servidor comprueba los permisos y es la fuente de verdad.
- **Sincronización local.** Dos dispositivos se emparejan con un código QR y se pasan los cambios por la red de casa.

## Las piezas

| Pieza | Carpeta | Qué es |
|---|---|---|
| La app | [`app/`](https://github.com/kosmotheka/kosmotheka/tree/main/app) | Una app de [Tauri 2](https://tauri.app) para escritorio y Android. La interfaz (React, TypeScript, Tailwind y shadcn/ui) está en `src/`; el núcleo, en Rust, en `src-tauri/` |
| El dominio | [`crates/domain/`](https://github.com/kosmotheka/kosmotheka/tree/main/crates/domain) | Tipos del dominio, validación, el reloj lógico híbrido y las operaciones. No sabe nada de bases de datos ni de pantallas |
| La especificación | [`spec/`](https://github.com/kosmotheka/kosmotheka/tree/main/spec) | El formato de las operaciones: un JSON Schema y casos de conformidad. Licencia MIT |
| El servidor | [`server/`](https://github.com/kosmotheka/kosmotheka/tree/main/server) | Opcional. PHP 8.5, Symfony 8, Doctrine y PostgreSQL, en Docker |
| La web y la documentación | [`web/`](https://github.com/kosmotheka/kosmotheka/tree/main/web) y [`site/`](https://github.com/kosmotheka/kosmotheka/tree/main/site) | La página kosmotheka.com y este sitio (Starlight) |

Cada parte lleva sus propias herramientas. Lo único que comparten la app y el servidor es `spec/`.

### Dónde va cada cosa

- **La lógica del dominio vive en Rust**, nunca en la interfaz. React llama a *comandos de Tauri* (funciones de Rust que la interfaz invoca) y no ejecuta SQL. *Por qué:* hay una sola implementación de cada regla, que además se prueba sin abrir una ventana.
- **Un catálogo, dos implementaciones de las reglas.** Las reglas de las operaciones están escritas en `spec/` y las cumplen dos programas: el núcleo Rust de la app y el servidor PHP. Los mismos casos de conformidad las ejecutan los dos, para que no se separen.
- **Los servicios de fuera son siempre opcionales.** Los catálogos de datos de libros, películas o juegos, y cualquier IA futura, van detrás de interfaces que se pueden cambiar. Si no responden, la app sigue funcionando.

## Qué pasa al añadir un libro

Sigamos un libro desde el botón hasta otro dispositivo.

1. **La interfaz llama a un comando.** Pulsas «Guardar» en la ficha. React llama a un comando de Tauri con los datos.
2. **El núcleo construye una operación.** Una **operación** es un lote atómico de cambios en las entidades del catálogo, con una etiqueta de intención (por ejemplo `book.add`). Añadir un libro crea, a la vez, la obra, la edición, sus detalles y el ejemplar. *Por qué:* si todo cambio es una operación, sincronizar es solo intercambiar operaciones, y el servidor puede comprobar los permisos sobre sus cambios. El historial de cada libro sale gratis del registro.
3. **Se guarda en la base local, en una transacción.** `ops::record` apunta la operación en el registro y aplica sus cambios en las tablas, todo junto: o se hace todo, o nada. La operación lleva una marca de un **reloj lógico híbrido** (una hora que nunca retrocede y que ordena los cambios de varios dispositivos aunque sus relojes no coincidan) y la firma del dispositivo.
4. **Los dispositivos emparejados se la piden.** Cuando dos dispositivos coinciden en la misma red, cada uno le dice al otro qué operaciones tiene y recibe las que le faltan. Quien las recibe las comprueba (firma, permisos) y las aplica con `sync::merge`, que acaba siempre en el mismo estado sea cual sea el orden de llegada.
5. **Con la nube, también el servidor.** Si la biblioteca está subida a un servidor, el dispositivo le envía la operación. El servidor comprueba la firma y los permisos de quien la hizo y la vuelve a aplicar sobre PostgreSQL. Lo que un dispositivo no podía hacer según su rol no se pierde: queda como una **petición** que quien administra acepta o rechaza.

Quien no use cuenta se queda en los pasos 1 a 4. Las operaciones con todo detalle, el motor de sincronización y los permisos son temas de las siguientes partes de esta documentación.

## Por dónde seguir

- [Preparar el entorno: la app](../setup-app/) y [el servidor](../setup-server/).
- [Cómo contribuir](../how-to-contribute/).
- [Glosario](../glossary/), para los nombres que se repiten en el código.
- [`AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/AGENTS.md) resume lo mismo para un asistente de programación; vale también para una persona.
