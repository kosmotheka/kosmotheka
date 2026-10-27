// Checks that spec/operation.schema.json agrees with the conformance cases.
// Rules a JSON Schema cannot express are skipped (see spec/README.md).
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";

const spec = fileURLToPath(new URL("../../spec/", import.meta.url));
const schema = JSON.parse(readFileSync(`${spec}operation.schema.json`, "utf8"));
const cases = JSON.parse(readFileSync(`${spec}conformance/operation-validation.json`, "utf8"));
const validate = new Ajv2020({ allErrors: true, strict: false }).compile(schema);

const beyondSchema = (expect, name) =>
  [
    "missing_field",
    "null_required",
    "other_library",
    "other_device",
    "wrong_id",
    "trash_pair",
  ].includes(expect.code) ||
  (expect.code === "wrong_type" && (name.includes("date") || name.includes("on the clock"))) ||
  (expect.code === "invalid_previous" && name.includes("its own previous"));

let mismatches = 0;
for (const { name, operation, expect } of cases) {
  const valid = validate(operation);
  const expectValid = expect.result === "valid";
  if (valid !== expectValid && (expectValid || !beyondSchema(expect, name))) {
    mismatches++;
    console.error(`MISMATCH: ${name}`, validate.errors?.[0] ?? "");
  }
}
// The operations of the merge and permission cases are valid operations too.
let checked = 0;
for (const file of ["library-merge", "merge", "permissions"]) {
  const mergeCases = JSON.parse(readFileSync(`${spec}conformance/${file}.json`, "utf8"));
  for (const { name, operations, expect } of mergeCases) {
    for (const { unsigned, ...operation } of operations) {
      checked++;
      if (!validate(operation)) {
        // An operation a case expects rejected may be one that is not even read: its `hlc` does not match
        // (a device in capitals, SEG-029). Only that, and only if the case says it is refused.
        const unreadable =
          (expect?.rejected ?? []).includes(operation.id) &&
          validate.errors.some((error) => error.instancePath === "/hlc");
        if (unreadable) continue;
        mismatches++;
        console.error(`MISMATCH: ${file}: ${name}`, validate.errors?.[0] ?? "");
      }
    }
  }
}
if (mismatches > 0) process.exit(1);
console.log(
  `spec schema agrees with ${cases.length} conformance cases and ${checked} merge and permission operations`,
);
