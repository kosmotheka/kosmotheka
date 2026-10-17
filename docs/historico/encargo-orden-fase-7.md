# Encargo · Ordenar la Fase 7 y callar el aviso de `schema:validate`

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push, sin cambiar de rama. Nada de funcionalidades nuevas (regla del maintainer mientras no pruebe).

## 1. El aviso falso de `doctrine:schema:validate`

`CloudOperation#body` es `\stdClass` a propósito: un `{}` vacío no puede volverse `[]`, porque cambiaría la firma. Doctrine lo marca como `[FAIL]` en el mapeo y confunde a quien despliega. Lo vio el maintainer en producción el 17/10/2026.

- Un tipo propio de Doctrine (`json_object`, por ejemplo) que guarde y lea un `\stdClass` tal cual, registrado como hace Symfony. Que el mapeo diga ese tipo.
- **Sin migración, si se puede:** la columna sigue siendo `JSON`/`JSONB` igual. Comprueba con `make console C="doctrine:schema:validate"` que salen los dos `[OK]`, y con `doctrine:migrations:diff` que no hay nada que migrar. Si hiciera falta una migración, para y dímelo.
- Los tests que leen o escriben `body` siguen pasando; añade uno que pruebe que `{}` y `{"a":{}}` vuelven como objeto.
- Quita el aviso de la guía (`site/…/run-your-server/troubleshooting.md`) y de `docs/plans/fase-7.md`.

## 2. Poner al día el estado de `docs/plans/fase-7.md`

La lista «Estado» tiene casillas sin marcar de cosas hechas (por ejemplo, 7.5.5c dice «encargo escrito y sin enviar», y está en `9e7b4a1`). Repásala contra `git log` y lo que dicen los mismos apartados:

- **Hecho y probado por el maintainer:** `[x]`.
- **Hecho y sin probar en pantalla:** se queda `[ ]`, pero con «hecho; falta la prueba en pantalla» al principio, para que se vea de un vistazo.
- **Sin hacer:** `[ ]`, igual que ahora.

Al final del apartado, una lista corta: **«Para cerrar la Fase 7»**, con lo que de verdad falta. Por lo que sé:

- las pruebas en pantalla de 7.3 (local), 7.5.5a, b y c, y 7.6;
- la prueba que la cierra: compartir con alguien de otra ciudad;
- la sesión de seguridad de 7.9;
- las copias automáticas en producción;
- Mercure en producción (opcional);
- `assetlinks.json` y los enlaces que se abren solos;
- GameUPC (espera su clave);
- la segunda pasada del maintainer por el servidor.

Comprueba cada punto antes de ponerlo. **No toques el texto histórico de cada apartado**: solo las casillas, el principio de cada línea y la lista final. En `docs/ROADMAP.md`, la Fase 7 tiene que decir lo mismo en resumen.

## Al terminar

`make check` una sola vez, al final y en segundo plano. `npm run build` en `site/`. Un informe corto.
