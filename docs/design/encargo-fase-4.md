# Encargo de diseño: bibliotecas (Fase 4)

> Segundo encargo para Claude Design, después del [brief](brief.md). Lo decidido está en D46, D54, D65 y D66 ([DECISIONS.md](../DECISIONS.md)). Todo en español, móvil (390 × 844) y escritorio (1280 × 800), claro y oscuro.

## Contexto

La app ya puede tener **varias bibliotecas** en un dispositivo («Familia», «Rol», «Videojuegos de Hugo»). Se trabaja con **una a la vez**, elegida en el selector de la barra superior, y cada biblioteca se comparte por separado con cada dispositivo.

Hay que seguir las pantallas que ya existen en `templates/mobile-screens` y `templates/desktop-screens`: la barra superior con el selector, la barra inferior de tres destinos, el menú lateral, los paneles que suben desde abajo en el móvil y los diálogos en escritorio. Los mensajes de error y de aviso siguen el patrón de [errors.md](errors.md): qué ha pasado, por qué suele pasar, qué hacer y un botón.

Todavía **no hay roles ni personas** (llegan en la Fase 5): hoy cualquier dispositivo que tiene una biblioteca puede hacer todo en ella. Las pantallas deben dejar sitio para que más adelante aparezcan «de quién es este dispositivo» y «con qué rol», pero sin mostrarlo aún.

## Pantallas

### 1. Selector de bibliotecas, con varias

La hoja (móvil) y el diálogo (escritorio) que ya existen, ahora con tres o cuatro bibliotecas: nombre, número de obras, estado de sincronización y la activa marcada. Abajo, «Nueva biblioteca».

### 2. Nueva biblioteca

Solo pide el **nombre**. Al crearla, la app pasa a ella y muestra su catálogo vacío (el estado vacío que ya existe).

### 3. Ajustes de una biblioteca

Una pantalla por biblioteca, a la que se llega desde el menú lateral o el selector:
- Cambiar el nombre.
- **Con qué dispositivos se comparte**, con la opción de dejar de compartirla con uno.
- **Quién la creó y cuándo** («Creada en el portátil el 26/09/2026»), en pequeño. Es una prueba: queremos ver si aporta o sobra.
- Al final, separadas: **Salir de la biblioteca** y **Borrar la biblioteca**.

### 4. Borrar una biblioteca

- Pide **confirmación** y avisa claramente de que **no se puede deshacer**, y de que se borra también **en todos los dispositivos** con los que se comparte (nombrarlos).
- **Ofrece hacer una copia de seguridad** de esa biblioteca antes de borrar, como acción destacada.
- Estado después: la app vuelve a otra biblioteca y lo dice con un aviso breve.

### 5. Salir de una biblioteca

Otra cosa que borrar: la biblioteca sigue para los demás y solo desaparece de este dispositivo. Confirmación que lo explique en una frase. Estado aparte para cuando llegue la Fase 5: **el último administrador no puede salir**; tiene que nombrar a otro o borrar la biblioteca.

### 6. Compartir: elegir qué bibliotecas van a otro dispositivo

- Al **emparejar** (mostrar el QR o escanear el del otro), elegir qué bibliotecas se comparten con ese dispositivo. La activa viene marcada; las demás no se envían hasta marcarlas. **Nada sale sin decirlo.**
- En la lista de dispositivos emparejados, ver y cambiar qué bibliotecas van a cada uno.
- Error a contemplar: el otro dispositivo tiene una versión de la app distinta y no se pueden sincronizar hasta actualizar.

### 7. Mover un libro a otra biblioteca

Desde la ficha (menú «⋮»): elegir la biblioteca de destino. Explicar en una frase qué pasa: la edición y sus ejemplares pasan a la otra biblioteca; el historial se queda en la de origen. Aviso breve al terminar.

### 8. Buscar en todas las bibliotecas

- El buscador tiene la opción «en esta biblioteca / en todas». En «todas», los resultados se agrupan por biblioteca.
- «¿Lo tengo?» ya mira siempre en todas: la pantalla «Lo tienes» con dos bibliotecas ya está diseñada; basta con revisarla.

## Qué entregar

Las pantallas de arriba en móvil y escritorio, claro y oscuro, en `templates/` como las anteriores, reutilizando sus piezas (barra superior, menú lateral, portadas por tipo).
