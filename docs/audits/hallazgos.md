# Hallazgos de auditoría

Registro vivo. Cada identificador es fijo y no se reutiliza. El procedimiento está en `README.md`.

| ID | Estado | Severidad | Título | Dónde | Auditor | Abierto | Cerrado | Informe |
|---|---|---|---|---|---|---|---|---|
| CAL-001 | descartado | baja | Parámetro `only` siempre vacío (falso: `notices` le pasa un filtro) | `requests.rs:91` | coordinador | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CON-001 | abierto | media | `library-merge.json` solo lo ejecuta Rust | `spec/conformance/library-merge.json` | facts | 2026-10-03 | | [2026-10-03](2026-10-03.md) |
| CON-002 | resuelto | media | `duplicate_library` registrado y sin pantalla | `lib.rs:149` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CON-003 | resuelto | baja | 177 traducciones sin uso aparente (solo 9 de verdad; el resto, claves dinámicas) | `app/src/i18n/locales` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CON-004 | resuelto | media | La lista de géneros del JSON Schema no es la de Rust ni la de PHP (anterior a la 6.7) | `spec/operation.schema.json` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| COS-001 | resuelto | baja | Búsquedas que no descartan respuestas viejas | `book-form.tsx:1347`, `series-work-picker.tsx:51`, `tags-page.tsx:31` | triaje ts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| PRU-001 | resuelto | baja | Test ignorado que no se llama `live_*` | `signature.rs` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-001 | resuelto | media | La copia automática de Android se lleva las claves | `AndroidManifest.xml` | threat-model | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-002 | resuelto | media | Volver a emparejar con un `device_id` conocido suplanta a ese dispositivo | `lan/server.rs:148-153` | threat-model | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-003 | resuelto | media | CI sin `permissions:` y con actions de terceros sin fijar | `.github/workflows/ci.yml` | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-004 | resuelto | media | Falta `SECURITY.md` | raíz | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-005 | resuelto | media | Sin avisos de licencias de terceros | `web/assets/fonts`, `app/package.json` | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-006 | abierto | baja | La CSP deja cargar imágenes de cualquier origen https | `tauri.conf.json` | threat-model | 2026-10-03 | | [2026-10-03](2026-10-03.md) |
| SEG-007 | resuelto | baja | En Windows la clave TLS se escribe sin permisos propios | `lan/identity.rs:109-112` | threat-model | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-008 | abierto | baja | Dependencias con avisos sin parche, fuera del uso de la app | `Cargo.lock`, `package-lock.json` | supply-chain | 2026-10-03 | | [2026-10-03](2026-10-03.md) |
| SEG-009 | resuelto | baja | Dockerfiles de desarrollo como root o con `sudo` | `server/docker/*/Dockerfile` | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
