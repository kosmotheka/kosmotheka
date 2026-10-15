// Checks what the second-factor fields accept (D100): the 6 digits of the authenticator app, or a backup code
// (XXXXX-XXXXX, the alphabet of server/src/Security/TwoFactor.php BACKUP_CODE_ALPHABET), with or without the dash,
// in capitals or not. Runs the TypeScript file as it is, with Node's own type stripping.
import { isSecondFactorCode } from "../src/lib/second-factor-code.ts";

const accepted = [
  "123456",
  " 123456 ",
  "ABCDE-FGHJK",
  "abcde-fghjk",
  "ABCDEFGHJK",
  "abcdefghjk",
  "23456-78923",
  "2345678923",
];
const refused = [
  "",
  "12345",
  "1234567",
  "12345a",
  "ABCDE-FGHJ",
  "ABCDE-FGHJKL",
  // 0, O, 1, I and L are not in the alphabet of the backup codes.
  "ABCDE-FGHIK",
  "0BCDE-FGHJK",
  "ABCDE-FGHJ1",
  "ABCDE_FGHJK",
  "ABCD-EFGHJK",
];

let wrong = 0;
for (const value of accepted) {
  if (!isSecondFactorCode(value)) {
    console.error(`should be accepted: ${JSON.stringify(value)}`);
    wrong++;
  }
}
for (const value of refused) {
  if (isSecondFactorCode(value)) {
    console.error(`should be refused: ${JSON.stringify(value)}`);
    wrong++;
  }
}
if (wrong > 0) {
  console.error(`${wrong} wrong`);
  process.exit(1);
}
console.log(
  `second-factor codes: ${accepted.length} accepted and ${refused.length} refused, as expected`,
);
