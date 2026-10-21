# Handoff: Milestone L0, life foundations (consent, store, evidence record, local-only, wellbeing basics)

For an AI coding agent working in the **rune-studio** repository. Read all of it first. The plan it comes from: `rune-life-harness-milestones.md` (in this folder; it explains the goal and the later milestones L1 to L10, which you must **not** start). Current behaviour: `docs/ARCHITECTURE.md`. Where this file and the code disagree about a fact, the code wins: say so in the report.

## 1. Goal

Studio's long-term goal is a **life harness**: it works from **evidence** (what the user says is stored as what they said, graded, and checked against what tools can observe), it does not assume, it expects that the user and the model can both be wrong, it can change a plan when evidence says it is failing, and it collaborates instead of automating everything. That needs sensitive data (goals, health, mind, money). **L0 builds the safe base before any such data is collected.** It delivers no conversation and no dashboard.

## 2. Decisions assumed (the owner can change them; say in your report if you departed)

1. The life module lives **inside Rune Studio** (`src-tauri/src/life/`), behind a clean boundary (a small public API; nothing else in Studio reaches into its tables), so it can be split out later.
2. A **database is the source of truth** (a separate file, not the agent trace). Obsidian will be a generated, editable view and an input later (L2); not in L0.
3. Data classes: `general`, `health`, `mind`, `finance`. Every record belongs to exactly one.
4. **Local-only is enforced and not overridable** for `health`, `mind` and `finance`.
5. Default state: the life module is **off**; every class consent is **off**.
6. No passive activity sampling, no screenshots and no screen recording as life evidence. Not in L0, and not by default later.

## 3. Rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop`, then `git push origin develop`.
2. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (all test commands, clippy, fmt) and again at the end.
3. Never claim a live behaviour you did not observe; test with unit tests, temporary databases and fixtures.
4. **No new dependency without writing the case in the report** (especially for encryption). The repository already uses a SQLite crate for the trace and a keyring for the bridge token: look at how they are used before choosing.
5. Never write life data to the **agent trace**, the session folders, the application log, panic output or any file outside the life database and an explicit export. Test this with a canary string.
6. Keep the Rune design language in any UI you touch (Oxanium, dot style, `#141414`, `#f1f1ef`, red only for failure and attention).

## 4. What is NOT CERTAIN (decide by a spike, say what you chose and why)

- **Encryption at rest.** Options: (a) encrypt sensitive columns with an authenticated cipher using a key kept in the OS keyring (a small crate), or (b) a whole-database encrypted SQLite build (a build feature whose behaviour on every CI target, including Windows, is unverified), or (c) rely on the OS account and disk encryption and document it. Do a short spike, pick one, justify it. Whatever you choose must survive: key loss (a clear message, no silent data loss), migration, and export.
- **Whether a small local model can detect distress reliably.** L0 does **not** claim that. See WP6.
- **Where rune-infer will report locality** (INF-15 asks for a `locality` field per model and an echo on responses; M1 adds a model profile with `locality`). Until then L0 uses the fallback in WP4.

## 5. Work packages

### WP1: module, store and migrations
- `life/` with its own database file in the app data folder (not the trace database), opened with the same care as the trace (WAL, busy timeout, a version number, a newer file is left untouched).
- Tables (names are suggestions; keep what is needed, nothing more):
  - `meta` (version, created, key id).
  - `consent` (`class`, `enabled`, `changed_utc`).
  - `records`: `id`, `class`, `kind` (claim, fact, preference, note, event), `body`, `source` (`user_said`, `user_detail`, `tool`, `import`), `grade` (0 to 3, see below), `occurred_utc`, `created_utc`, `supports_json` (links to the evidence behind it), `deleted` (tombstone for undo within a short grace period only).
  - `assumptions`: `id`, `text`, `class`, `status` (`untested`, `supported`, `refuted`), `evidence_json`, `created_utc`, `changed_utc`.
  - `plans` and `plan_versions` (empty in L0): `id`, `version`, `body_json`, `reason`, `evidence_json`, `created_utc`, `approved` (so L6 has a place to land).
  - `audit`: `id`, `at_utc`, `actor` (user, system, a named tool), `action`, `subject`, `detail`. **Not** a hash chain: user deletion must always work, and a chain would fight it.
- **Evidence grades**, as constants with doc comments: 0 your word only; 1 your word with a specific checkable detail; 2 observed by a tool or imported data the user did not type; 3 independent and hard to fake. A function checks that a record's source and grade are consistent (a `user_said` record can never be grade 2 or 3; a `tool` or `import` record cannot be grade 0).
- Migrations are additive and tested (v1 creates; a test opens a fixture of v1 with the current code).

### WP2: consent, retention, export, delete (the user's control)
- Settings fields: `lifeEnabled` (false), per-class consent (false), per-class `retentionDays` (0 means keep).
- Writing a record of a class whose consent is off is **refused** with a clear error (tested for every class). Turning consent off does not delete; it stops collection.
- Retention sweeps expired records of that class (tested with a fake clock); it never touches other classes.
- **Export:** one function writes a class or all classes to a folder as JSON plus a readable Markdown file per kind, including grades and sources. **Delete:** removes a class or everything, including tombstones and the audit rows about the deleted records' content (the fact that a deletion happened stays, with no content), then vacuums so the data is not left in the file. Tested: after delete, a search of the file bytes for a canary string finds nothing (also in the WAL after a checkpoint).
- CLI (`rune-studio-cli`): `life status` (enabled, consents, counts, database size, encryption mode), `life export <class|all> <folder>`, `life delete <class|all> --yes` (refuses without `--yes`), `life audit [--limit n]`. Same style and exit codes as the existing commands.
- A small **Settings section "Life data"** in the existing settings window, in the Rune style: the master switch, one row per class (consent, retention), Export and Delete buttons (Delete asks for confirmation with the class name typed or the existing confirm pattern). Plain text explaining what is stored, that it never leaves the machine for health, mind and finance, and that Rune is not a clinician. The text is placeholder copy marked `OWNER TO REVIEW`.

### WP3: encryption at rest
Implement the option chosen by the spike (section 4). Requirements: no plaintext of a sensitive class on disk (check the file bytes with a canary after a write, a checkpoint and a close); the key is not in the database file, the settings file or the logs; losing the key gives a clear error and offers "start fresh" only after an explicit confirmation; a backup copy of the database file is useless without the key (state this in the user text). Report the cost (write and read times for 10,000 records) and the dependency added, if any.

### WP4: local-only enforcement (the central rule)
- One function `may_send(class, endpoint) -> Allow | Deny(reason)` in `life/`, the **only** place that decides. Rules: `general` may go to any endpoint the user has configured; `health`, `mind` and `finance` may go **only** to an endpoint that is local.
- "Local" is decided in this order: (1) the model's `locality` from rune-infer or Studio's model profile (M1), when present: `local` allows, `cloud` and `lan` deny, `unknown` denies; (2) otherwise, a **fallback by address**: the base URL's host is a loopback address (`127.0.0.0/8`, `::1`, `localhost`); anything else denies. Never decide by model name. Resolve the host properly (no substring tricks: `http://127.0.0.1.evil.com`, `http://localhost@evil.com`, userinfo, ports and IPv6 forms must all be handled; tests for each).
- The model client gets a way to tag a request with a data class (a field on the request); if tagged and `may_send` denies, the client returns an error **before** sending anything. Untagged requests behave as today. A test with a fake server proves that no byte reaches a non-local server for a tagged request.
- A response that echoes a locality of `cloud` for a request that was allowed (a fallback or an alias hid a cloud model) is treated as a violation: log it, drop the response, and tell the caller (tested with a fake response header; the header name is **NOT CERTAIN**, so isolate it).
- Document that this protects data going to the model **server**, and that rune-infer's own cloud adapters are rune-infer's side of the same rule (INF-15).

### WP5: redaction in logs and the existing record
- A redaction function for life content used by every log call in `life/`; the application log, the trace and the flight recorder never contain life record bodies (test with a canary through the whole path). Errors from the module carry codes and ids, not content.

### WP6: wellbeing basics (small, honest)
- A static module with the text a user sees when a conversation touches distress: fixed, human-written, never model-generated. Ship **placeholder text marked `OWNER TO REVIEW`** and a place to put region-specific resources; do not invent hotline numbers.
- A conservative phrase matcher (a plain list in a data file, easy to extend) and a function `wellbeing_check(text) -> None | Pause`. It is **not wired to any conversation yet** (L1 does that). Tests with a fixed phrase set, including negatives.
- State in the code comment and in the user text that this is **not a safety guarantee**, that a small local model is not trusted to detect distress, and that Rune is not a clinician.

### WP7: docs and tests
- `docs/ARCHITECTURE.md` (the life module and its rules), `README.md` (Privacy: what the life module stores, the local-only rule), a user-facing `docs/life-data.md` in plain language (what is stored, where, how to export and delete, what is never collected), and the privacy note you can finish only with the owner's text.
- Final numbers; clippy and fmt clean.

## 6. Do not

No conversation, no strategy session, no dashboard, no Obsidian export (L1 to L2). No passive sampling, no screenshots as evidence. No cloud path for health, mind or finance, ever. No hash chain. No telemetry. No new network calls. Do not store life data in any existing store.

## 7. Report

`docs/handoff/L0-report.md`: baseline and final numbers; the encryption spike (options, what you measured, what you chose, the dependency added if any, key-loss behaviour); per work package what changed and its tests; the canary tests and their results; the list of address forms tested in WP4; live steps for the maintainer (turn the module on, grant one class, write and read a record through the CLI test helper, export, delete and confirm the bytes are gone, point the base URL at a non-local address and confirm a finance request is refused); every decision you changed from section 2 and why; every NOT CERTAIN with what you did.
