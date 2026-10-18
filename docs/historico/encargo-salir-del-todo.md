# Encargo · «Salir de la biblioteca» del todo: retirarse uno mismo el acceso (D102, parte 2)

> Encargo cerrado para la sesión de hitos, escrito el 18/10/2026. Sin commit, sin push. **Empieza cuando termines la parte 1** («Quitar de este dispositivo» y «Volver a bajar», [encargo-salir-con-nube.md](encargo-salir-con-nube.md), con el nombre nuevo) y me hayas mandado su informe. Lee D102 entera, D98, D73 y SEG-032 (`docs/audits/`) antes de empezar. **Toca quién recibe una biblioteca y una regla de permisos de `spec/`: el test hostil va primero, y lo tienes que ver fallar.**

## Qué

1. **La regla nueva, en `spec/`:** quien no administra puede escribir `access_withdrawn = true` en **su propia** persona (`member`): la persona a la que está asignado el dispositivo que firma la operación. Nada más cambia:
   - retirar el acceso a **otra** persona sigue siendo solo de quien administra;
   - **quitarse** la marca (`false`) sigue siendo solo de quien administra (D98, SEG-032);
   - el último administrador no puede retirárselo (la regla que ya existe para echarle);
   - un dispositivo sin persona no puede.

   Busca dónde se decide hoy quién puede escribir `access_withdrawn` y los casos de conformidad de D98 (`spec/conformance/`). Añade los casos nuevos ahí, y que **los pasen la app (Rust) y el servidor (PHP)**. Si para escribir la regla hace falta un campo nuevo o subir `schemaVersion`, para y dímelo.
2. **Lo que pasa al aplicarse:** lo mismo que cuando echa quien administra (D98). La operación quita la persona de cada dispositivo suyo, con su valla, y crea su retirada; esos dispositivos borran la biblioteca al conectar, y la nube corta a la cuenta. **Que use el mismo código que el de echar, no una copia.** Si el de echar lleva datos que solo puede escribir quien administra, dime cómo lo resuelves antes de escribirlo.
3. **En el propio dispositivo:** al aplicar su salida, la biblioteca se borra de ese dispositivo al momento, como una retirada (y por eso nunca ofrece «Volver a bajar»).
4. **La pantalla:** en la página de la biblioteca, junto a «Quitar de este dispositivo», la acción **«Salir de la biblioteca»**, con un diálogo que diga, sin jerga:
   - que desaparece de todos tus dispositivos y de tu cuenta;
   - que tus libros siguen en la biblioteca, a tu nombre;
   - que volver solo lo puede decidir quien administra.

   Pide confirmación escribiendo el nombre de la biblioteca, si es lo que hace la app con borrar; si no, lo mismo que haga borrar. Al último administrador no le deja: le dice que nombre a otro administrador o que borre la biblioteca.

## Tests, en este orden

1. **Hostiles primero, fallando:**
   - quien no administra no puede retirar el acceso a otra persona, ni por la red local ni por la nube (en la app y en el servidor);
   - nadie que no administra puede quitarse la marca a sí mismo;
   - un dispositivo sin persona no puede retirársela a nadie;
   - el último administrador no puede salir.
2. **Salir:**
   - funciona para un editor y para un lector;
   - sus otros dispositivos la borran al conectar, por la red local y por la nube;
   - la cuenta deja de recibirla (servidor);
   - la persona sigue en el catálogo con sus ejemplares y sus marcas.
3. **Convergencia:** un test en `sync.rs` con `assert_converged`: la salida de una persona mientras otro dispositivo edita.
4. **Conformidad:** los casos nuevos de `spec/conformance/` los ejecutan Rust y PHP.

## Novedades y guía

- Una línea bajo `"0.3.0"` en `app/src/whats-new.json`, junto a la de «Quitar de este dispositivo» o en la misma.
- `several-libraries.md`, `people-and-permissions.md` y `the-cloud.md`, en `es/` y `en/`.
- `docs/architecture/` donde esté la regla de D98 (`sync.md`, `security.md` o donde corresponda) y `spec/README.md` si enumera las reglas.

## Al terminar

- En `app/`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:i18n`, `check:whats-new` y `check:spec`.
- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.
- En `server/`: `make fix` y **un solo `make check`**, en segundo plano.
- En `site/`: `npm run build && npm run check:links`.

El informe:
- la regla, con los casos de conformidad;
- cómo reutiliza el código de echar;
- los tests, y que los hostiles fallaron antes;
- cómo se ve la acción.
