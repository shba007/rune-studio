# Generated release preparation and artifact delivery

## Owner-authorized historical formatting

Historical formatting uses the same pinned changelogen 0.6.2 renderer as new
releases. Rust `history-import` retains legacy entries and comparison baselines,
rejects unrecognized content and conflicting duplicate versions, and collapses
only identical sections after newline normalization. Historical wording and IDs
are retained; normal new-release validation remains strict.

For an explicit formatting migration, save a GitHub release API snapshot and run:

```sh
bun scripts/format-release-history.mjs --releases SNAPSHOT.json
```

This prepares a generated changelog, exact per-version release bodies, original
changelog backup and a hash/asset manifest in the ignored
`temp/studio-release-format/` directory. Add `--write-changelog` only when the owner
has requested the history migration. `--source PATH` can re-render the original
backup. The command never changes GitHub, versions or Git refs. An authenticated,
explicitly authorized metadata update may replace existing release bodies with
those exact files; check source-body hashes before writing and verify release
identity/assets afterwards. Missing historical releases are not created.
Ordinary preparation still preserves history and restricts `prepare --apply`
to trusted develop CI.

## Source of truth and version policy

`changelogen@0.6.2` is pinned in package.json and bun.lock. It alone parses
Conventional Commits, calculates the bump, groups changes and renders Markdown.
There is no custom bump algorithm or handwritten release renderer in xtask.
`changelog.config.mjs` explicitly configures features, fixes/performance and all
maintenance groups. Features advance minor (including below 1.0); fixes/performance
and substantive maintenance advance patch. Breaking changes override type: major
after 1.0, minor before 1.0. Changelogen's pre-1.0 default would demote features to
patch; the small adapter configures its package-major classification for those
features so the package produces the required minor bump. Fixtures enforce this.

All valid semantic types, including custom types, are eligible. Routine ci/chore
release bookkeeping, apply-code-fix commits and internal planning labels are
excluded. Substantive fix(release), docs, refactor, tests, build and CI changes remain.
Changelogen renders commit/PR links, compare links, breaking markers and contributors.
Contributor names come from Git; optional handles come from a reviewed local map.
Online author enrichment is disabled so outages/account changes cannot change a
repeat preview. No AI service or network prose generator is involved.

## Workflow and ownership

1. Owner commits/pushes develop. Only develop pushes start Vue CI; merge/release
   commits still receive strict frontend, app Rust and release-integration checks.
   PR events and release-branch pushes do not start duplicate integration runs.
2. Successful validation permits trusted Actions preparation. It checks the tested
   source is current, runs the pinned package, synchronizes package/Tauri/Cargo,
   writes the generated changelog and preparation metadata, and validates all
   generated files in that same run. It rechecks develop immediately before PR update.
3. One release/update-version → develop PR is created/updated. Both title and
   generated commit are ci(release): update version. The extracted version section
   is its body. The owner reviews the generated files and successful source run.
4. Owner merges the preparation PR (merge or squash supported), then promotes
   develop → main with a merge commit preserving source ancestry. Ordinary merge
   titles work; no ci(release) marker is required. Merging develop alone never publishes.
5. Every main push validates versions, exact prepared notes/digest, generator,
   baseline and source ancestry. An already published version is a successful
   explicit no-op. An invalid, decreasing or stale pending preparation fails with
   instructions to prepare through develop again. A valid unpublished preparation
   publishes its matching tag/release using the already prepared notes file.
6. release.published triggers Vue CD. All platforms check out that tag and run
   the same Rust exact-section validator before builds/uploads to the existing
   release ID. No publication or artifact step regenerates prose from Git history.

Agents only implement/test locally and leave work unstaged/uncommitted. Agents do
not edit CHANGELOG.md or run actual preparation on the working repository. CI owns
release file generation and branch/release mutations; the owner reviews/promotes.
The generator's bump operation runs in a temporary scratch package during previews,
so local calculation writes neither the repository changelog nor manifests.

CI runs each test suite once. Preparation checks only the generated files and
extracts PR notes; it does not rerun the source test suite. Rust build caches are
reused between validation, preparation and publication where the OS matches.

## Exact note contract and repeat runs

Rust xtask supplies the single section extractor to preparation, publication and
artifact validation. The first version heading must uniquely match the manifests.
Missing, empty, duplicate and placeholder sections fail. The release body omits the
version heading and includes the exact substantive body, compare/commit/contributor
links and groups. Only CRLF → LF and surrounding blank newline delimiters are
normalized. No Markdown rewriting, mention/link removal, trimming of meaningful
spaces, commit-range regeneration or unrelated-note fallback is accepted.

Preparation always considers the full source range after the actual previous tag,
using changelogen for the greatest pending bump. An unpublished section may be
regenerated; all published historical suffix bytes are preserved. Repeat unmerged
runs produce the same version/section. `.release/prepared.json` records generator,
sourceHead, baseTag, version and notesHash. A merged preparation with no new source
changes produces no PR/bump. New source recomputes the pending section from the
same published baseline, without duplicate version sections. Source freshness
checks plus publication's unconsumed-source refusal prevent stale publication.

## Local read-only commands

```text
bun install --frozen-lockfile
bun run release:prepare
bun run release:preview
bun run release:test
cargo test --locked --manifest-path xtask/Cargo.toml
cargo run --locked --manifest-path xtask/Cargo.toml -- notes
```

xtask prepare/example delegate to Bun and remain supported. `sync VERSION` is small
manifest integration, not a bump calculator; it never edits the changelog. Actual
`prepare --apply` is guarded to trusted develop CI. Tests use temporary fixtures,
including a read-only Git object reference; they create no commits, tags or branches.

## Authentication and repository settings

Enable Actions to create pull requests. The preparation job grants Contents/Pull
requests write. `GITHUB_TOKEN` is used for PR API operations; event-capable `GH_TOKEN`
is used for branch pushes. PR validation is the original preparation run, not a bot
PR event. Release creation also uses GH_TOKEN so release.published can trigger CD;
a release created with GITHUB_TOKEN alone would not provide that downstream event.
GH_TOKEN must be a usable PAT or refreshed App installation token with repository
Contents write. Keep existing Tauri signing secrets and relevant branch protection.
Values need not be exposed. Live token/settings and publishing are owner checks.

## Failed v0.0.27 release diagnosis and recovery

[Run 40060814562](https://github.com/shba007/rune-studio/actions/runs/40060814562)
failed on all four platforms at Validate exact prepared release body, before artifact
build/upload. Public job annotations confirm exit 1 at that step. Full log downloads
require authenticated access (public API returned 403); the first failure was reproduced
with the tagged source and validator:

- package.json at tag v0.0.27 says 0.0.27; Cargo.toml and Cargo.lock say 0.0.26.
  validate_versions runs first, producing "versions must agree".
- Additionally, the published contributor text is `(@shba007)`, while the tagged
  changelog contains `([@shba007](https://github.com/shba007))`. Those are different
  Markdown bodies, not a line-ending difference, so version synchronization alone
  would not repair note validation. The old publisher independently regenerated
  notes using changelogen while artifacts compared a stored custom-tool contract.

The working manifests now match existing package 0.0.27, without modifying the
repository changelog. No published body/tag/history was rewritten. Owner recovery:
commit/push this repair to develop, review the generated preparation PR, merge it,
and promote with an ordinary ancestry-preserving merge. The next release will use
one prepared section end to end. Rerunning the old immutable tag still uses its old
inconsistent files and is not a substitute for a correctly prepared release.

No later artifact/build failure was observed in that run: those stages never ran.
Linux/macOS installers and live GitHub dispatch remain unverified locally. Existing
PipeWire/SPA, Clang, WebKit/GTK, GBM/EGL, XCB, libxdo and ALSA dependencies are retained.
Report any failure after note validation separately; do not relax the note contract.