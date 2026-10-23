# Release preparation and publication

## Owner and CI responsibilities

Agents implement/test locally and leave changes unstaged/uncommitted. The owner
commits and pushes development changes. GitHub Actions may prepare/push the
release branch and publish after owner review/promotion. No AI service is used.

1. Owner pushes develop. Vue CI runs frontend typecheck/tests/build/lint/format,
   release utility tests/Clippy/format, and Windows app Rust tests/Clippy/format.
   All PRs (including release and promotion PRs) get the same checks; there is no
   blanket skip for merges or release commits. PR jobs have read-only permissions.
2. After both validation jobs pass, a trusted develop push prepares release files.
   It checks the tested SHA is still current, then uses the Rust utility to update
   package.json, src-tauri/Cargo.toml, src-tauri/Cargo.lock, CHANGELOG.md and
   .release/prepared.json. Tauri already reads ../package.json. bun.lock contains
   dependency metadata, not the app version; xtask is an independently versioned,
   unpublished developer utility. No other source files are autoformatted/committed.
3. peter-evans/create-pull-request updates one branch, release/update-version,
   targeting develop. Both title and commit are ci(release): update version.
   The exact generated changelog section supplies its PR body. Owner reviews
   versions, notes, diff and successful Frontend and release utility / Windows Rust
   checks. Merge or squash this PR; do not squash unrelated development changes
   into the bookkeeping subject before preparation.
4. Owner opens develop → main promotion PR. Use Create a merge commit, preserving
   ancestry (do NOT squash/rebase promotion). Set the merge commit title to
   ci(release): publish vX.Y.Z, using the prepared package.json version. This
   marker on the main HEAD is required by Vue Release. Merging the preparation
   PR into develop alone NEVER publishes. Squashing promotion would also make
   previously consumed source commits reappear in later tag-to-develop ranges.
5. Vue Release validates synchronized versions and extracts the exact matching
   latest CHANGELOG section. Missing, mismatched, empty, duplicate or placeholder
   notes fail. It refuses an existing tag, then creates vX.Y.Z and its release
   with that extracted body, using GH_TOKEN.
6. That release.published event starts Vue CD. Each platform validates release
   tag/body against the same Rust extractor, then uploads signed installers and
   CLI assets to the existing release ID. It does not regenerate release prose.
   Owner checks all platform jobs, Windows installer download and latest.json.

## Deterministic rules and repeat runs

The utility uses git-conventional for parsing, semver for versions and toml_edit
for Rust manifest/lock edits. All valid semantic commit types are releasable,
including docs, refactor, style, test, build, ci, chore, revert and custom types.
Other than feat/fix/perf or breaking changes, they appear under Maintenance and
use a patch bump. Merge wrappers, routine release/version/autoformat bookkeeping,
placeholders and internal planning labels are excluded. Write public descriptions
in reviewed conventional commit subjects; the tool does not invent product prose.
Under the repository's existing pre-1.0 progression nonbreaking changes bump patch,
breaking changes advance minor. After 1.0, feat bumps minor and breaking bumps major.

Tags supply the last published baseline. An already prepared pending version is
reused (or advanced if new breaking changes require it). An existing unpublished
changelog section is retained and new entries are deduplicated. Published sections
are preserved byte-for-byte. .release/prepared.json stores the consumed source
head; a merged preparation with no new substantive commits produces no PR. An
unmerged preparation rerun uses the same develop inputs, therefore the same
version/notes/branch. New reviewed changes update that one outstanding PR.
The source head must remain an ancestor; unexpected history rewrites fail clearly.

## Local commands (no Git mutations or publication)

```text
cargo test --locked --manifest-path xtask/Cargo.toml
cargo clippy --locked --manifest-path xtask/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path xtask/Cargo.toml -- prepare
cargo run --locked --manifest-path xtask/Cargo.toml -- example
cargo run --locked --manifest-path xtask/Cargo.toml -- notes
```

prepare prints a proposal; it never changes files without --apply. The apply flag
is used by CI/owner only. Neither mode stages, commits, pushes, tags or publishes.
example uses synthetic reviewed messages without creating commits or files:

```markdown
## v0.0.24

### Fixes

- kit: report incompatible tool gateways clearly
```

Repeating that example yields the same output. The repository currently has a
pending 0.0.23 section. Preparation reuses it and includes the reviewed refactor
commit that the earlier narrow type filter excluded. The example above is not
a promised actual release version.

## Authentication and repository settings

GH_TOKEN already exists in the repository workflow contract. It MUST be a usable
PAT or GitHub App installation token whose events trigger downstream workflows,
with repository Contents and Pull requests write access. A stored short-lived App
installation token must be refreshed by the repository's App integration; do not
assume an expired static token works. A classic PAT needs repo access (and workflow
scope if GitHub requires it for branch ancestry containing workflow changes).
Fine-grained PAT/App access must include this repository and required organization
approval/SSO. Never print token values. Both preparation and publication fail
clearly if the secret is missing; actual scopes/expiry require owner verification.

Do not replace GH_TOKEN with the default GITHUB_TOKEN: bot-created PR/release events
have special triggering/approval restrictions, so validation/artifact delivery
cannot be assumed. The explicit event-capable token is used for create-pull-request
and gh release create. Artifact uploads can use GITHUB_TOKEN because they do not
need to trigger another workflow. No secret is supplied to pull-request validation.

Repository/org policy must allow Actions, the listed third-party actions, bot
release branches/PRs and the token permissions. Protect develop/main with the
Frontend and release utility and Windows Rust checks (use the exact observed
check names after the first owner-run CI). Allow merge commits for promotion;
keep owner review and any required approval protections. Permit only the release
actor to push release/update-version. Existing updater signing secrets remain
TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD. This change
neither changes secrets/settings nor bypasses protections.

The full PR → merge → main → release → artifact event sequence was reviewed
locally, not executed. Only owner actions can verify token policy, protections,
actual GitHub event delivery and all installer platforms. No live run was dispatched.

## Remaining non-Rust tooling

The superseded scripts/release-notes.mjs and its tests are removed after equivalent
Rust tests passed. GitHub Actions YAML, third-party Actions, git/gh CLI transport,
and small shell orchestration remain; application Vue/TypeScript/Bun tooling is
unchanged. End users install prebuilt products and never need Rust.

Inspected scripts/m1-system-test.ps1, m1-target.ps1, eval/run.ps1 and
m15/run-probe.ps1. They have no release/version/changelog logic to consolidate:
they manage existing Windows desktop/WinForms fixtures, explicit live evaluation
and probe setup. Their Node browser rendering/reporting is existing evaluation
infrastructure. Rewriting these unrelated GUI/evaluation harnesses would be a
large product-testing change, so they remain; no obsolete release PowerShell or
Python implementation is retained.

## Local verification for this change

Windows: nine release-tool tests passed, including repeat preparation/consumed
source state, historical preservation, exact extraction, synchronized manifests,
no-change exclusions, semantic versions and event/body mismatch refusal.
483 application Rust tests passed (11 environment-dependent tests ignored);
160 frontend tests passed across 22 files. A separate real Studio → Kit test passed
with local Kit 0.5.0, time/git 0.3.0 and filesystem 0.4.1: all five operations,
root enforcement, plugin error handling, deselection and recovery. Typecheck,
production build, lint, both Rust Clippy runs, formatting, YAML syntax and diff
whitespace checks passed. Host incremental hard-link fallback warnings were
non-fatal. The local dry run changed no Git state and now includes the previously excluded refactor. The example and idempotence tests need no commits.

No live workflow, PR, tag or release was created. Repository settings/token scopes,
Linux/macOS execution and live model/GUI acceptance are untested here. Temporary
Kit registry data stayed under this repository, without modifying installation.
Suggested owner commit subject: fix(ci): restore automated release preparation.