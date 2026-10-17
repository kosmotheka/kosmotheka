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

## Cómo funciona por dentro

6. [El motor de sincronización, explicado](./sync-engine/): cómo varios dispositivos llegan al mismo resultado sin que nadie mande.
7. [Las operaciones: la especificación y los casos de conformidad](./operations-spec/): el formato, las reglas y cómo escribir un cliente o un servidor compatible.
8. [Seguridad: el modelo de amenazas](./security/): de quién protege, con qué piezas y qué no protege.
9. [Cómo auditamos, para repetirlo por tu cuenta](./audit-method/): el método, no los informes.
10. [Avisar de un fallo de seguridad](./report-a-vulnerability/): cómo hacerlo en privado.

## Los datos, las cuentas y la nube

11. [El modelo de datos](./data-model/): las entidades del catálogo y de compartir, y por qué la obra y la edición van separadas.
12. [Cuentas e inicio de sesión](./accounts/): registro, Google, segundo factor y el dispositivo como sesión.
13. [La nube: la API y las reglas de acceso](./cloud-api/): un mapa de las rutas y de quién llega a qué biblioteca.

## Publicar

14. [Publicar una versión](./release/): el número, las novedades, las licencias y el paquete firmado de Android.

## Documentos del repositorio

- [CONTRIBUTING.md](https://github.com/kosmotheka/kosmotheka/blob/main/CONTRIBUTING.md), el procedimiento y las reglas que importan (en inglés).
- [SECURITY.md](https://github.com/kosmotheka/kosmotheka/blob/main/SECURITY.md), cómo avisar de un fallo de seguridad en privado.
- [CODE_OF_CONDUCT.md](https://github.com/kosmotheka/kosmotheka/blob/main/CODE_OF_CONDUCT.md).
- [El registro de decisiones](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md) y [la arquitectura](https://github.com/kosmotheka/kosmotheka/blob/main/docs/architecture), en español.
