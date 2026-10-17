---
title: Colaborar
description: Cómo ayudar con Kosmotheka y cómo está construido.
sidebar:
  order: 0
---

Se colabora con el procedimiento de siempre: abrir un *issue* si el cambio es grande, hacer *fork* y una rama, pasar las comprobaciones y abrir un *pull request*. Esta parte explica cómo está construido el proyecto, para que puedas entenderlo, arreglar algo o añadir algo.

Está pensada para quien programa y no conoce el proyecto. Se usan términos técnicos (operación, reloj lógico híbrido, conformidad), pero cada uno se explica la primera vez que sale, y siempre se dice por qué se hizo así.

## Para empezar

1. [Las piezas y cómo encajan](./overview/): la app, el dominio, la especificación, el servidor y qué pasa al añadir un libro.
2. [Preparar el entorno: la app](./setup-app/): Rust, Node, Android y las comprobaciones.
3. [Preparar el entorno: el servidor](./setup-server/): Docker, las bases de datos, las fixtures y `make check`.
4. [Cómo contribuir: estilo, pruebas y commits](./how-to-contribute/): las reglas que importan y cómo añadir un campo o una entidad.
5. [Glosario](./glossary/): cómo se llama cada cosa en español y en el código.

## Documentos del repositorio

- [CONTRIBUTING.md](https://github.com/kosmotheka/kosmotheka/blob/main/CONTRIBUTING.md), el procedimiento y las reglas que importan (en inglés).
- [SECURITY.md](https://github.com/kosmotheka/kosmotheka/blob/main/SECURITY.md), cómo avisar de un fallo de seguridad en privado.
- [CODE_OF_CONDUCT.md](https://github.com/kosmotheka/kosmotheka/blob/main/CODE_OF_CONDUCT.md).
- [El registro de decisiones](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md) y [la arquitectura](https://github.com/kosmotheka/kosmotheka/blob/main/docs/architecture), en español.

## Lo que viene después

Todavía se están ordenando para esta documentación el motor de sincronización, las operaciones y la especificación con sus casos de conformidad, el modelo de seguridad y el método de auditoría. Hasta entonces, están en la carpeta [`docs/architecture`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/architecture) del repositorio.
