// Checks that spec/operation.schema.json agrees with the conformance cases.
// Rules a JSON Schema cannot express are skipped (see spec/README.md).
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";

const spec = fileURLToPath(new URL("../../spec/", import.meta.url));
const schema = JSON.parse(readFileSync(`${spec}operation.schema.json`, "utf8"));
const cases = JSON.parse(
  readFileSync(`${spec}conformance/operation-validation.json`, "utf8"),
);
const validate = new Ajv2020({ allErrors: true, strict: false }).compile(schema);

const beyondSchema = (expect, name) =>
  ["missing_field", "null_required", "other_library", "other_device", "wrong_id"].includes(expect.code) ||
  (expect.code === "wrong_type" && name.includes("date")) ||
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
  const mergeCases = JSON.parse(
    readFileSync(`${spec}conformance/${file}.json`, "utf8"),
  );
  for (const { name, operations } of mergeCases) {
    for (const { unsigned, ...operation } of operations) {
      checked++;
      if (!validate(operation)) {
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
