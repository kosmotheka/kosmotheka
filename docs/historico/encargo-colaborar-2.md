# Encargo · Documentación para quien colabora, segunda tanda: cómo funciona por dentro

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Mismas reglas que la primera tanda** ([encargo-colaborar-1.md](encargo-colaborar-1.md)): léelas antes, y mira cómo quedaron `overview.md` y `how-to-contribute.md`.
> - Solo `site/src/content/docs/es/contribute/`.
> - `library-haiku` traduce a la vez en `en/contribute/`: no lo toques.
> - Sin secretos.
> - Sin nada del servidor oficial.
> - Sin fechas de trabajo, hitos ni sesiones.

## Las páginas

1. **`sync-engine.md` · El motor de sincronización, explicado.**
   - Qué es local primero.
   - Las operaciones y el registro.
   - El reloj lógico híbrido.
   - La fusión campo a campo y por qué siempre converge.
   - Las lápidas.
   - Cómo dos dispositivos se piden lo que les falta.
   - Las firmas y la cadena de cada autor (`previous`).
   - Qué hace el servidor cuando lo hay.
   - **Una sección «Por qué así y no de otra forma»**, que lo compare con otros enfoques: WebRTC con relé, sincronizar archivos, un servidor central, CRDT de documento.

   Material: `docs/architecture/sync.md`, `docs/architecture/operations.md`, `docs/research/sync-engines.md` y las decisiones que enlacen (D25, D31, D84, D87, D94…).

   **Es la página que más importa de esta parte:** que alguien que no conoce el proyecto la entienda de una lectura. Con un ejemplo que siga dos dispositivos que cambian el mismo libro sin conexión.
2. **`operations-spec.md` · Las operaciones: la especificación y los casos de conformidad.**
   - El formato (`spec/operation.schema.json`).
   - Las intenciones.
   - Las reglas que comprueba quien recibe.
   - Los casos de conformidad, y cómo los ejecutan Rust y PHP.
   - Cómo añadir un caso.
   - Que `spec/` es MIT para que cualquiera escriba un cliente o un servidor compatible.

   Material: `docs/architecture/operations.md`, `spec/` y `docs/architecture/data-model.md`.
3. **`security.md` · Seguridad: el modelo de amenazas.**
   - De quién nos protegemos y de quién no.
   - Qué protege cada pieza: el emparejamiento por QR, las firmas, los roles comprobados al recibir, el servidor que no se fía de ningún id del cliente, la retirada de acceso.
   - Y lo que **no** protege: el servidor puede leer lo que se sube; un dispositivo robado y desbloqueado.

   Material: `docs/architecture/security.md` y `docs/audits/amenazas.md`.

   **Ningún hallazgo concreto de las auditorías**, ni abierto ni cerrado: solo el modelo.
4. **`audit-method.md` · Cómo auditamos, para repetirlo por tu cuenta.**
   - La parte determinista: escáneres y scripts en Docker con versión fijada, qué sale y cómo se lee.
   - La parte de juicio: temas, preguntas, gravedad y verificación, que puede seguir una persona o cualquier asistente, no solo Claude.
   - Lo que aún no cubre.

   Material: `docs/audits/README.md` y la estructura de `.claude/skills/audit/` (los temas, las reglas y lo que hace cada script de `bin/`).

   **Se publica el método, no los informes ni los hallazgos** (D90).
5. **`report-a-vulnerability.md` · Avisar de un fallo de seguridad.** Corta: lo de `SECURITY.md`, en llano, con el aviso privado de GitHub.

Y **`index.md`**: añade estas páginas y quita de «Lo que viene después» lo que ya está.

## Lo que pesa

- **Todo comprobado en el código de hoy.** Los documentos de arquitectura son de trabajo y alguno puede haberse quedado atrás: si no coinciden con el código, manda el código, y apúntalo en el informe.
- **Términos del glosario.** Usa los de `glossary.md` y enlázalo la primera vez. Las palabras internas (valla, sello) se pueden usar aquí, porque es la parte técnica, pero explicadas, y solo si hacen falta para entender.

## Al terminar

- `cd site && npm run build && npm run check:links`.
- **El informe:**
  - cada página con su número de palabras;
  - lo que no coincidía entre los documentos de trabajo y el código;
  - lo que dejaste fuera por ser interno o por ser un hallazgo.
