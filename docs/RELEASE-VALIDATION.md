# Release repair validation — 2026-10-24

## Observed failure

Public run [40060814562](https://github.com/shba007/rune-studio/actions/runs/40060814562)
stopped all four artifact jobs at exact prepared-body validation, before building.
Tagged v0.0.27 had package 0.0.27 but Cargo manifest/lock 0.0.26; the old validator
rejects that first. Independently, the published contributor `(@shba007)` differs
from the stored `([@shba007](https://github.com/shba007))`. Public annotations and
tagged sources were inspected; full job-log downloads returned HTTP 403.
Neither historical release nor tag nor repository CHANGELOG was rewritten.

## Implementation and results

Pinned changelogen 0.6.2 now owns parsing, bumping and Markdown generation.
The JavaScript adapter is necessary to call its npm APIs; Rust remains responsible
for exact extraction, manifest synchronization and event validation. Frontend
Bun/Vue tooling, Actions YAML and existing desktop/bootstrap fixtures remain.
No AI service is used. Reviewed offline author mapping makes contributor lookup
deterministic without independent prose generation.

Locally passed on Windows:

- 498 application Rust tests; 12 existing live/system tests ignored.
- 167 frontend tests across 27 suites; frontend build and typecheck.
- 5 Rust release tests and 5 package-adapter test groups covering policy fixtures,
  greatest breaking precedence, historical preservation, repeat preparation,
  stale preparation, ordinary main promotion titles and exact artifact notes.
- Application and xtask Clippy with warnings denied; Rust formatting, frontend and
  adapter formatting, strict lint and git diff whitespace checks.
- Real-repository preparation preview: no releasable commits at current tagged
  HEAD. Publication evaluation: successful already-published no-op.

Initial frontend tests failed to read a sandbox temporary module cache; the same
suite passed outside that sandbox. No product code was changed. Actual live
capture/vision inference and a real Kit server were not rerun; their existing
application regression checks passed. Linux/macOS native builds and installers,
live GitHub permissions and publication remain owner/CI verification.

The same Rust extractor serves generated PR notes, main publication and the
unchanged four-platform artifact validator. CRLF/LF and surrounding newline
transport delimiters are normalized; links, text and meaningful spaces must match.
The historical contributor mismatch still fails as intended.

## Exact read-only fixture preview

Feature plus fix after 0.0.27 produces the following package-generated section.
This is a fixture, not a proposed manual edit to the repository changelog.

```markdown
## v0.1.0

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.27...v0.1.0)

### 🚀 Enhancements

- **insight:** Explain a selected window ([abcdef1](https://github.com/shba007/rune-studio/commit/abcdef1))

### 🩹 Fixes

- **kit:** Report HTTP connection errors ([abcdef2](https://github.com/shba007/rune-studio/commit/abcdef2))

### ❤️ Contributors

- Example Contributor
```

Fix/maintenance from 0.0.27 yields 0.0.28; feature/breaking yields 0.1.0.
From 1.2.3, fix/maintenance yields 1.2.4, feature 1.3.0 and breaking 2.0.0.
Reapplying an identical temporary preparation produces no bump or duplicate section.

## Owner delivery

Commit/push development changes to develop, review/merge the single generated
release/update-version PR, then promote develop to main with an ordinary merge
commit preserving ancestry. No special title is required. Every main push checks
prepared metadata, synchronized versions, source freshness and notes digest;
already-published versions are a successful no-op. Publishing uses the extracted
prepared body and an event-capable GH_TOKEN so release.published triggers artifacts.
Enable Actions PR creation and preserve token/signing settings described in
RELEASING.md. Do not rerun the inconsistent immutable old tag expecting this repair.

Studio guidance now uses ~/.rune; Kit/Infer own migration and explicit overrides.
Studio moves no installation data and does not own their server processes.
Repository CHANGELOG SHA256 remains
430C06AD9EE19343DB8E54E1CEF53E0F19BCEA9320E65CCCF7FFCC82E8C68303.
No later artifact/build failure was observed: the failed release never reached it.

Suggested owner commit subject: fix(release): unify generated notes and artifact validation