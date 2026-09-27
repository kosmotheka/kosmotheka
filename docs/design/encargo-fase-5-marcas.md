# Encargo de diseño: marcas por persona (Fase 5, hito 5.8)

> Cuarto encargo de la Fase 5 para Claude Design, 27/09/2026. Lo decidido está en D68 ([DECISIONS.md](../DECISIONS.md)) y los detalles de construcción en [plans/fase-5.md §5.8](../plans/fase-5.md). Todo en español, móvil (390 × 844) y escritorio (1280 × 800), claro y oscuro.

## Contexto

Cada persona de la biblioteca apunta **lo suyo** sobre cada obra, sin pedir permiso a nadie y sea cual sea su rol (también quien solo lee):
- **Estado**, con la fecha en que lo puso: pendiente, leyendo, terminado o abandonado. Sin progreso: nada de «voy por la página 120».
- **Valoración** de 1 a 5 estrellas, sin medias.
- **Favorito**, aparte de la valoración: sirve para filtrar, y un favorito puede tener menos estrellas que otros.

Las marcas son de cada persona: lo que marca Hugo no cambia lo de Laura. Quien administra puede ver las de todos.

Por ahora solo hay libros; en la Fase 6, con los demás tipos, «leyendo» pasará a «jugando» o «viendo». El diseño debe dejar sitio para eso, sin mostrarlo aún.

## Pantallas

### 1. Lo mío, en la ficha de la obra

En la ficha (el panel de la derecha en escritorio, la pantalla completa en móvil), donde hoy está la etiqueta «Leído» junto a «Lo tienes», un bloque **«Lo mío»**:
- El estado, en un solo toque: pendiente, leyendo, terminado, abandonado. Quitarlo también.
- La fecha, que se pone sola (hoy) al cambiar el estado y se puede corregir.
- Las estrellas y el favorito.
- En móvil, que se pueda marcar «terminado» sin abrir nada más.

Estados: sin marcar, leyendo, terminado con 4 estrellas y favorito, y abandonado.

### 2. Lo de los demás

En la misma ficha, una línea discreta con lo que han marcado las otras personas («Laura lo terminó en marzo · ★★★★★»), que se despliega si son varias.

### 3. Filtrar por lo mío

En los filtros del catálogo: por mi estado (pendiente, leyendo, terminado, abandonado, sin marcar) y «Mis favoritos». En la lista y la tabla del catálogo, un indicador pequeño de mi estado en cada obra, sin recargar la fila.

### 4. Sin persona

Un dispositivo que no es de ninguna persona no puede marcar: el bloque «Lo mío» se ve desactivado, con una línea que diga que primero alguien que administra tiene que decir de quién es el dispositivo.
