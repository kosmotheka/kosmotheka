---
agent: audit-reviewer-strong
code:
  - app/src-tauri/src/lan/*.rs::@prod
  - app/src-tauri/src/commands.rs::*pair*|*device*|*sync*|*lan*|*server*
context:
  - docs/architecture/sync.md
  - facts/surface.md#AndroidManifest.xml
decisions: D13 D34 D37 D56 D67 D71
max_lines: 2600
---
Device identity, pairing by QR and the LAN transport.

1. **Identity** (`identity.rs`): algorithm, source of randomness, where private keys and certificates are stored and with which file permissions, what the identity proves.
2. **Pairing** (D13, D34, D67): what the QR contains; what it grants (D67: a QR or link must not grant more than intended; proving identity must not grant permission); expiry and single use; can a photo or screenshot of the QR be used later?; is the channel bound to the QR (certificate fingerprint pinned, D34) so a device in the middle cannot join?; do both people confirm?; how the client secret is stored (hash?).
3. **Transport**: the address the service listens on (all interfaces or one) and when it runs; authentication of **every** request after pairing; what an unpaired or withdrawn device gets before rejection (names, libraries, counts); replay protection.
4. **Robustness**: limits on body size, number of operations, time and connections; any `unwrap`, indexing or unbounded allocation on received bytes (any device on the LAN could crash the app or exhaust memory).
5. **Rediscovery** (D37): scanning the subnet: what it sends, to whom, and whether a hostile device can impersonate a known one.
6. **Android**: cleartext traffic and permissions that the LAN needs, from the manifest facts.
