# Encargo · Jerga de la segunda tanda y textos de la app

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Va con la revisión de jerga de `library-haiku` ([revision-jerga-2.md](revision-jerga-2.md)) y con lo que el maintainer respondió a las diferencias de la segunda tanda. **Solo textos:** ni pantallas nuevas ni cambios de comportamiento. Toca `site/` y las cadenas de `app/src/i18n/locales/`.

## 1. Aplicar la revisión de jerga a las seis páginas

Con el mismo criterio que en la primera tanda: lo que proponga el informe y alargue mucho la frase o se equivoque sobre la app, no se aplica, y se dice por qué en una línea. «La nube» y «Fuera de casa» se reescriben por partes.

Dos correcciones de fondo que el informe señala:
- **`delete-account.md`:** «se borra al momento» (desde la app) frente a «30 días como mucho» (por correo) parecen contradecirse. Son dos caminos distintos: dilo así.
- **Quién se queda con una biblioteca compartida** al borrar la cuenta (D99): primero una cuenta cuya persona administra y, si no hay, cualquier otra con acceso. Dilo en llano.

## 2. Los textos de la app que respondió el maintainer

- **«Antes de que existieran las firmas»** (`cloud.errors.unsigned_history`, `cloud.pick.unsigned`): **reescríbelos en lenguaje accesible**, en `es` y en `en`, diciendo qué le pasa a la persona y qué puede hacer, sin la palabra «firma». Mira en el código cuándo salen, para que el texto acierte.
- **La Ayuda «Si pierdes el móvil» y «Fuera de casa»** (`help.topics.lost`, `help.topics.away`): **rehazlas** para la app de hoy, con y sin cuenta, en `es` y en `en`. Cortas: es la Ayuda, lo largo está en la guía. Que no se confundan «Olvidar dispositivo» y quitar el acceso.
- **Perder el móvil con el segundo factor y sin los códigos de reserva:** el maintainer confirma que, de momento, se escribe a `privacy@kosmotheka.com` desde el correo de la cuenta. Que lo digan igual la guía y, si la Ayuda lo menciona, la Ayuda.

## 3. Las cuatro formas de «quitar» un aparato: fase 0, sin aplicar

El maintainer quiere unificar el criterio. Hoy hay cuatro nombres casi iguales:

- «Olvidar dispositivo», en «Compartir y dispositivos»;
- «Quitar un dispositivo», en Personas;
- «Quitar», en «Mis dispositivos» de la cuenta;
- «Dejar este dispositivo a otra persona», en Ajustes.

**Propón, sin cambiar nada:**
- qué hace de verdad cada una, mirado en el código: a quién afecta, si se puede deshacer, qué se borra y dónde;
- un nombre y una frase de ayuda para cada una, que digan lo que pasa y no se parezcan entre sí;
- cómo quedaría el texto de confirmación de cada una.

Una tabla: hoy · qué hace · nombre propuesto · frase · confirmación. El maintainer decide antes de cambiar nada.

## Al terminar

`npm run build` en `site/`; en `app/`, `typecheck`, `lint` y `format:check`. Informe: filas aplicadas y no aplicadas, los textos de la app cambiados (antes y después) y la tabla de la parte 3.
