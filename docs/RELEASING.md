# Release preparation and publication

## Owner and CI responsibilities

Agents implement/test locally and leave changes unstaged/uncommitted. The owner
commits and pushes development changes. GitHub Actions may prepare/push the
release branch and publish after owner review/promotion. No AI service is used.

1. Owner pushes develop. Vue CI runs frontend typecheck/tests/build/lint/format,
   release utility tests/Clippy/format, and Windows app Rust tests/Clippy/format.
   Only develop pushes trigger Vue CI. PR events and release/update-version pushes
   do not trigger duplicate runs. Merge/release commits still receive validation.
2. After both validation jobs pass, a trusted develop push prepares release files.
   It checks the tested SHA is still current, then uses the Rust utility to update
   package.json, src-tauri/Cargo.toml, src-tauri/Cargo.lock, CHANGELOG.md and
   .release/prepared.json. Tauri already reads ../package.json. bun.lock contains
   dependency metadata, not the app version; xtask is an independently versioned,
   unpublished developer utility. The same run validates generated version/notes,
   reruns release-tool tests and checks locked Cargo metadata before opening the PR.
   No other source files are autoformatted/committed.
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
with repository Contents write access for branch pushes and release publication.
PR API operations use the job's GITHUB_TOKEN with explicit Pull requests write
permission. The observed PAT could push the release branch but PR creation failed
with Resource not accessible by personal access token; the two credentials now
have separate roles. A stored short-lived App
installation token must be refreshed by the repository's App integration; do not
assume an expired static token works. A classic PAT needs repo access (and workflow
scope if GitHub requires it for branch ancestry containing workflow changes).
Fine-grained PAT/App access must include this repository and required organization
approval/SSO. Never print token values. Both preparation and publication fail
clearly if the secret is missing; actual scopes/expiry require owner verification.

create-pull-request uses token=GITHUB_TOKEN for PR operations and
branch-token=GH_TOKEN for Git pushes. Vue CI listens only to develop pushes;
opening/updating the PR or pushing its branch starts no additional Vue CI run.
Source validation and generated-release-file checks happen in the original
develop run. Preparation itself still runs only on develop.
Keep GH_TOKEN for gh release create: publication must trigger release.published
and artifact delivery. Artifact uploads use GITHUB_TOKEN because they do not need
to trigger another workflow. Source validation jobs receive no release secrets.

Repository policy was read and already allows Actions to create/approve PRs;
no settings or secret values were changed. Live success of the credential split
requires the owner's next commit/push; rerunning an old SHA uses its old workflow.

Repository/org policy must allow Actions, the listed third-party actions, bot
release branches/PRs and the token permissions. The owner reviews the successful
develop run and generated-file checks before merging. With develop-only CI,
required PR-head checks would never arrive; the owner must align branch protection
with this trigger policy rather than waiting for a second PR workflow.
Allow merge commits for promotion;
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

## Restored workflow and Linux dependency correction

The active workflows again follow the flow above, using xtask rather than deleting
CHANGELOG or regenerating publication prose. Both before and after preparation,
the tested develop SHA is checked against origin/develop; an advanced branch is
left to its newer validated run. Generated files are validated in that same run.
The Ubuntu installer job installs libpipewire-0.3-dev, libspa-0.2-dev, libclang-dev,
GBM/EGL, XCB, libxdo and ALSA headers alongside WebKit/GTK. This fixes the reported
libspa-sys pkg-config failure; changing PKG_CONFIG_PATH cannot replace absent
system development packages. Linux installation/build still needs owner CI verification.

The pre-existing package version was 0.0.26 while Cargo remained 0.0.23; Cargo's
manifest/lock have been synchronized to the existing package version without
rewriting historical changelog entries. Release preparation remains CI-owned.
Local prepare currently proposes 0.0.27 from reviewed committed history; it excludes
uncommitted Screen Insight changes until the owner commits them. The synthetic
example remains deterministic at 0.0.24. Owner promotion still requires a merge
commit titled ci(release): publish vX.Y.Z, followed by release.published artifact
jobs. GH_TOKEN, Actions PR creation permission and signing secrets remain required.