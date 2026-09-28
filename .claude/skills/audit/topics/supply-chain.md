---
agent: audit-reviewer
code:
  - LICENSE
  - .gitignore
  - server/.gitignore
context:
  - facts/scanners.md
  - facts/surface.md#Environment files
  - facts/surface.md#.github/workflows/ci.yml
decisions: D03 D04 D11 D64 D72
max_lines: 1600
---
Dependencies, secrets, CI, licenses and readiness to publish (D03: the repository becomes public later).

1. **Vulnerabilities** (facts): for each, shipped to users or only development/build (the facts classify it), and the fix. Shipped with severity 7.0 or more is `alta`; RustSec "unmaintained" is `baja` with an alternative if the advisory names one. Group what one upgrade fixes.
2. **Secrets** (gitleaks facts): real secret (`crítica`: rotate it and clean the history before publishing; never rewrite history yourself) or a development value that is harmless by design (say why). `.env.local` and `.env.*.local` must be ignored.
3. **Licenses** (D11: AGPL for everything, permissive license for the spec): is each flagged license compatible with distributing the app under AGPL? MPL-2.0 is file-level copyleft. Fonts and icons: is their license recorded?
4. **CI** (`ci.yml` in the facts): top-level `permissions:` (least privilege), actions pinned by tag instead of commit SHA, untrusted input in `run:`, artifacts anyone can download and run without a signature.
5. **Ready to publish**: tracked files that must not be public (the `.gitignore` files shown, databases, keystores); a `SECURITY.md` explaining how to report a vulnerability.
