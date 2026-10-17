# Encargo · Documentación para quien colabora, tercera tanda: datos, cuentas y nube

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Mismas reglas que las dos tandas anteriores** ([encargo-colaborar-1.md](encargo-colaborar-1.md) y [encargo-colaborar-2.md](encargo-colaborar-2.md)):
> - solo `site/src/content/docs/es/contribute/`;
> - sin secretos;
> - nada del servidor oficial;
> - sin fechas de trabajo, hitos ni sesiones;
> - ningún hallazgo de auditoría;
> - todo comprobado en el código de hoy.

## Las páginas

1. **`data-model.md` · El modelo de datos.**
   - Las entidades del catálogo y cómo se relacionan: obra, edición, ejemplar, persona, serie, marca, etiqueta y las tablas de detalle por tipo.
   - Las de compartir: biblioteca, miembro, dispositivo, retirada.
   - Por qué la obra y la edición van separadas.
   - Los ids, únicos dentro de su biblioteca.
   - Un diagrama en texto o una tabla, lo que se lea mejor.

   Material: `docs/architecture/data-model.md`, `crates/domain/src/schema.rs` y el glosario.
2. **`accounts.md` · Cuentas e inicio de sesión.**
   - Registro con correo y código.
   - Entrar con Google.
   - El segundo factor y los códigos de reserva.
   - El dispositivo como sesión, con su clave.
   - Los límites de peticiones.
   - Borrar la cuenta.
   - Qué guarda el servidor.

   Material: `docs/architecture/accounts.md`.

   **Ojo con el segundo factor al entrar con Google:** escribe lo que hace el código (`GoogleController`), no lo que diga una decisión.
3. **`cloud-api.md` · La nube: la API y las reglas de acceso.**
   - Las rutas de `/v1`, agrupadas: cuenta, nube, invitaciones, metadatos y avisos.
   - Qué hace cada una, en una línea.
   - La regla de que el servidor no se fía de ningún id del cliente (D84).
   - De dónde sale el acceso a una biblioteca.
   - Subir y bajar operaciones.
   - Los avisos en directo (Mercure, opcional).

   Material: `docs/architecture/cloud.md` (largo: resume, no copies) y `bin/console debug:router` (`make console C="debug:router"`). **No es una referencia de cada campo:** es un mapa, que enlaza al código para el detalle.

Y **`index.md`**: añade las tres páginas al grupo que les toque, y quita la línea de lo que aún no está.

## Al terminar

- `cd site && npm run build && npm run check:links`.
- **El informe:**
  - cada página con su número de palabras;
  - lo que no coincidía entre los documentos y el código;
  - lo que dejaste fuera.
