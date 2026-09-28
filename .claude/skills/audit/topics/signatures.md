---
agent: audit-reviewer
code:
  - crates/domain/src/signature.rs::@prod
  - server/src/Sync/OperationSignature.php
  - server/src/Sync/InvalidSignatureException.php
  - server/src/Enum/SignatureErrorCode.php
context:
  - facts/contracts.md#Conformance cases
decisions: D56 D58 D73
max_lines: 1500
---
Operation signatures (D56), in Rust and PHP.

1. Algorithm and key formats; the exact list and order of signed fields. If the library, the device, the HLC or the operation id are not signed, an operation can be replayed in another library or attributed to another device.
2. Canonical form: key order, whitespace, number format, escaping of non-ASCII, null against missing fields, unknown fields. Do Rust and PHP produce the **same bytes**? Any difference makes valid signatures fail on one side.
3. **Parser differentials**: PHP's `json_decode` keeps the last of two duplicate keys; `serde_json` may reject them or keep another. If the bytes verified and the data applied can differ, a payload's signed meaning and applied meaning can differ.
4. Verification: constant-time comparison (libsodium verify, `hash_equals`); algorithm fixed, never chosen by the payload; what happens with unsigned or badly signed operations; binding between key and device.
5. Do both sides run the signature conformance vectors (see the facts)?
