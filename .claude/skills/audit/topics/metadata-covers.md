---
agent: audit-reviewer
code:
  - app/src-tauri/src/metadata/*.rs::@prod
  - app/src-tauri/src/covers.rs::@prod
  - app/src-tauri/src/commands.rs::*lookup*|*cover*|*isbn*|*burst*
decisions: D18 D26 D27 D62
max_lines: 1800
---
External metadata providers and cover downloads. AGENTS.md and D18: external services are always optional and the app must keep working without them.

1. Offline and failure: provider down, slow, returning an error page, empty or malformed JSON, or a huge body. Timeouts, a maximum response size, and what the person sees.
2. Parsing: any `unwrap`, indexing or assumption on the provider's JSON that can panic.
3. Covers: HTTPS only; certificate checks never disabled; redirects to local network addresses; a maximum size before writing to disk; decoding untrusted images; file names derived from content hashes (D27) and never from provider data.
4. What is sent: ISBN, API key, user agent. Is a Google Books key (D26) stored and sent safely, and never in the source?
5. Privacy: providers learn which books the family owns and their IP address; is the lookup clearly optional?
