---
title: "How we audit, so you can repeat it yourself"
description: "The project's audit method, in two parts: scripts that give facts, and judgement reviews of prepared code packets."
sidebar:
  order: 9
---

From time to time the project is audited: quality, contracts between layers, tests, data integrity and security. This page explains **the method**, so that you can repeat it on your copy, with the tool you prefer: a person, or any programming assistant, not only the one we use.

**The method is published, not the reports or the findings.** What an audit finds is fixed before it can interest anyone; the procedure, on the other hand, serves anyone who wants to review their own copy or a change they are preparing.

## The idea

An audit mixes mechanical work and judgement work, and it is best not to mix them:

1. **The mechanical part is done by scripts, with no model and no judgement.** They take a fixed copy of the code, run scanners and calculate checkable facts ("do these two lists match?"). They cost almost nothing and always give the same result.
2. **The part that needs judgement is done by whoever audits** (a person or a model), **over a code packet already prepared**: the questions, the decisions that apply and only the code that matters. Nobody explores the whole repository.
3. **Everything that is claimed is verified** against the code before it goes into a report.

*Why this way:* exploring a whole repository is the most expensive and least reliable thing. With a closed packet, the work is reproducible, cheap and can be shared out.

## What you need

- A copy of the repository with Git.
- **Docker.** All the tools run in containers with a fixed version: nothing is installed on your machine.
- `python3` on your machine (standard library only) to prepare the packets.
- Someone (or something) to do the reviews: a person, or the model you want.

The scripts and rules are in [`.claude/skills/audit/`](https://github.com/kosmotheka/kosmotheka/tree/main/.claude/skills/audit): `bin/` with the scripts, `topics/` with the topics, and `review-rules.md` and `triage-rules.md` with the rules for the reviews. The folder is named like that because the maintainer launches it with a Claude Code command (`/audit`), but **nothing in the method depends on that**: it is text, scripts and Docker.

## Part 1: the mechanical part

The script `bin/prepare.sh` does, in about 20 seconds:

1. Uses `git archive` to take a **fixed copy** of the latest commit of `main`, so as to always audit something specific. If you give it the commit of the previous audit, it also calculates what has changed since then.
2. Runs the scanners, **all in Docker with a fixed version**:

| Tool | Version | What it does | What leaves the machine |
|---|---|---|---|
| osv-scanner | v2.6.0 | Dependencies with known flaws (Rust, npm, Composer) and licences | Names and versions of the dependencies, to osv.dev and deps.dev |
| gitleaks | v8.30.1 | Secrets in the Git history | Nothing |
| semgrep | 1.177.0 | Static analysis of PHP, TypeScript, Rust, Docker, nginx and GitHub Actions | Nothing; it only downloads the rules |
| knip | 6.39.0 | Unused exports, files and dependencies in the app | Nothing; it only downloads knip |
| python | 3.13.15 | Runs `facts.py` | Nothing |

3. Runs `bin/facts.py`, which writes **checkable facts** into a folder, with a summary (`SUMMARY.md`) that marks what deserves attention:
   - that the migration schema, `schema.rs`, the JSON Schema and the PHP mirror **match**;
   - that each Tauri command the interface calls exists, and with the same arguments;
   - each **closed list** (formats, types, roles, states, genres) and each family of error codes, compared between Rust, TypeScript and the translations, according to `bin/contracts.json`;
   - the fields of each Rust `struct` against the TypeScript `interface` of the same name;
   - the English translations against the Spanish ones, and the conformance cases against whatever runs them;
   - the inventory of tests and the attack surface (CSP, permissions, Android, Docker, environment variables);
   - the **candidates for a problem**: `unwrap`, hand-built SQL, lost errors, React effects…

### How to read it

- Only `SUMMARY.md` is opened, and from there **only the sections it marks** (a warning, a number that is not zero).
- A mechanical difference between two lists **is not dismissed as "a script error" without comparing the two real sources by hand.**
- The facts turn into findings directly: a list missing a value in TypeScript is a medium-severity contracts problem; a field that the interface reads and Rust no longer sends is high severity.
- The candidates for a problem **are not findings yet**: they go through the triage in part 2.

## Part 2: the judgement part

### Triage: is this really a problem?

The mechanical candidates are many and almost all are false positives (an `unwrap` on a constant, for example). Triage decides which are real. Each **triage packet** carries up to 40 candidates, each with its piece of code (the whole function if it is short), and **the rules of its category**. For example:

- An `unwrap` on data that comes from another device, from the server or from the network is a serious problem (a faulty peer could bring the app down); on a local file or a user input, a minor one; on a constant, it is nothing.
- SQL built with `format!`: it is a problem if a **value** is interpolated; it is not if only table or column names that come from constants are interpolated.
- A write to a catalog table outside where it should be (`ops.rs` and `sync.rs`) is always serious, because it skips the rule that every write is an operation.

Whoever does the triage can read **only their packet**. They reply with a table: which candidates are findings and why, which are not, and which are "inconclusive" (those where the piece of code is not enough to decide).

### Review by topics

Each **topic** is a broad question with its own packet. The topics, by order of risk:

| # | Topic | What it reviews |
|---|---|---|
| 1 | `sync-convergence` | That devices converge: clock, deletions, batches, different app versions |
| 2 | `library-access` | Who can end up with the data of a library |
| 3 | `permissions-on-receipt` | That whoever receives checks the permissions and the withdrawal of access |
| 4 | `lan-pairing` | Identity, pairing by QR and transport on the local network |
| 5 | `ipc-surface` | Tauri commands, permissions, CSP, keys and Android |
| 6 | `data-files` | SQLite, backups, restoring with sync, CSV |
| 7 | `migrations` | Migrations that can lose data or leave the database half done |
| 8 | `signatures` | Signatures in Rust and PHP: the same bytes, differences between JSON parsers |
| 9–10 | `flows-sharing`, `flows-catalog` | Seams between functions: offline roles, requests, moving books, marks |
| 11 | `webview-rendering` | Untrusted data shown in the interface |
| 12 | `server` | The Symfony server: security and conventions |
| 13 | `metadata-covers` | Data services and covers, offline or with hostile responses |
| 14–15 | `spec-coverage`, `tests-quality` | That each rule has cases, and whether the tests would detect a failure |
| 16–20 | `ux-*` | Screens with no way out, errors, empty states, texts |
| 21–22 | `quality-rust`, `quality-ts` | Quality |
| 23 | `docs-vs-code` | Decisions and roadmaps against the code |
| 24 | `supply-chain` | Dependencies, secrets, licences, continuous integration |

One whole topic is reviewed per pass, rotating, and a topic comes back sooner if its code has changed. The threat model (`threat-model`) is written once and reviewed only when security decisions change.

Each topic is a file in `topics/` with a header (which code to include, which context documents, which decisions apply, the maximum size) and a list of **concrete questions**. The script `bin/packet.py` turns that file into a **packet**: the instructions, the questions, the decisions, the facts and only the code of the functions involved, with line numbers. For example, the questions of `sync-convergence` ask for reasoning with concrete timelines: "devices A, B and C, the operations each one makes, the order in which they arrive and the final state of each. A timeline that ends in different states, or a change lost without warning, is a finding".

### The rules for whoever reviews

They are summarised in `review-rules.md`. The most important:

- **Work from the packet.** Answer each question from the code it brings, without exploring the repository. At most eight extra checks, only to confirm or discard a specific hypothesis, and always read-only.
- **Accepted decisions are the project's intent.** What a decision chooses is not a finding, unless the code contradicts it or there is a risk that the decision does not mention.
- **No citation, no finding.** Each finding cites the exact code with its path and its lines. "X is missing" requires proof: say which code was checked.
- **One cause, one finding**, and at most twelve per topic.
- **A concrete scenario:** who does what, in which state, and what the wrong result is.

### Severity and confidence

| Severity | When |
|---|---|
| Critical | In normal use or for a realistic attacker: data loss or corruption that cannot be undone, devices that no longer converge, someone reads or writes what their role does not allow, or a real secret exposed |
| High | A feature or a guarantee breaks in a realistic case, or there is a security flaw with a plausible condition: same network, stolen phone, a member acting in bad faith, a modified app |
| Medium | Rare cases, missing defences, important rules without tests, debt that will soon cause failures |
| Low | Clarity, names, cleanup |

When in doubt between two levels, **the lower one**, saying what would raise it. The confidence can be **confirmed** (the path was traced in the code), **probable** (one step depends on something at run time) or **possible** (a suspicion, which is never critical or high).

### Verification

An audit full of false positives is worse than none. So:

- **No finding goes in if its citation is not in the audited code.**
- Critical and high findings are checked by hand: the citation is found and some lines around it are read. The scenario is confirmed, or downgraded or discarded with the reason. If it depends on something at run time, it stays as "probable" with concrete steps to check it on screen.
- Duplicates are searched for between reviews and against the findings already recorded.
- What an accepted decision chooses moves to "accepted" with its number; what needs a new decision goes to a "To decide" list. **The audit never writes to the decision log.**
- What was discarded is noted with its reason, so that it does not come back.

### What is recorded

Each finding has a fixed identifier that is never reused, with a prefix according to its family: security, data integrity, seams between functions, contracts between layers, tests or quality. The states are `open`, `resolved`, `accepted` (with the decision) and `discarded`. Each pass writes a report with what is new, what was resolved, what is still open, what needs a decision, manual test scenarios and what is pending. The audit **fixes nothing**: whoever maintains the project decides what is fixed and when.

## Repeating it on your own

1. **Prepare.** From the root of the repository, run `.claude/skills/audit/bin/prepare.sh <repo> <work-folder> [previous-commit]`. It takes about twenty seconds.
2. **Read the facts.** Open only `facts/SUMMARY.md` and the sections it marks. Turn the mechanical facts into findings, checking the two sources by hand.
3. **Choose the work.** A topic from the table (the next by risk, or the one your change touches) and, if you want, a group of candidates for triage.
4. **Build the packet.** `python3 .claude/skills/audit/bin/packet.py review <folder> <topic>` (or `triage <folder> "<categories>"`). It prints the path and the size. With `--incremental` it only includes what changed since the previous audit.
5. **Give it to whoever reviews.** The packet is a single text file with everything needed: you can read it yourself, or give it to any assistant with the instruction to follow it exactly and answer in the format it asks for. It does not need access to the repository.
6. **Verify.** Follow the verification rules above for each answer before accepting it.
7. **Write and clean up.** Note what you found wherever you prefer, and delete the work folder (it is a copy of the code).

One piece of advice: **audit little and often.** Two topics well reviewed per pass are worth more than twenty half done, and that is why the method has a budget: at most two topic reviews and three triage packets per pass.

## What it does not yet cover

- **What is seen on screen** (hidden texts, contrast in dark mode, buttons outside the safe area): it needs a visual review with screenshots, which is not built yet.
- **Whether the tests would detect a failure** (mutation tests, which change the code on purpose to see whether any test complains): they were done by hand once, and they are not part of the method or of continuous integration.
- **Running tests:** the method reads code and scanner output; it does not launch attacks against a running server or fuzzing. What depends on something running stays as "probable", with steps to test it by hand.
- **An external audit.** This is a systematic review done by the project's maintainer and by models, not an independent audit or a penetration test. A model can be wrong: that is why everything is verified against the code.
