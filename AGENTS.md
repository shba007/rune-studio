# Working agreement

AI agents implement/test locally and leave changes unstaged and uncommitted.
Agents must not commit, push, merge, tag, publish or dispatch publishing workflows.
Preserve unrelated work; do not modify sibling repositories.

GitHub Actions IS authorized to calculate versions, update manifests/lockfiles/
changelogs, commit/push the dedicated release/update-version branch, open/update
the release PR and publish tags/releases/artifacts after owner promotion. The
human owner commits/pushes development work and reviews/merges release PRs.
Do not use AI services in release generation.

Use pinned changelogen 0.6.2 for conventional-commit interpretation, version
calculation and changelog rendering. Configuration is in changelog.config.mjs;
scripts/release.mjs is package/state integration, not another renderer. The small
Rust xtask only synchronizes manifests and extracts/validates exact note sections.
Do not directly author, edit or patch CHANGELOG.md. CI generates it from reviewed
source commits. Local previews and temporary fixtures are permitted.

Commands from the repository root:

- bun run release:prepare (read-only)
- bun run release:preview (package-generated fixture example)
- bun run release:test
- cargo test --locked --manifest-path xtask/Cargo.toml
- cargo run --locked --manifest-path xtask/Cargo.toml -- notes
  The older xtask prepare/example commands delegate to the pinned package adapter.
  Actual --apply preparation is restricted to trusted develop CI. No command stages,
  commits, pushes or publishes; those actions belong to the workflow/owner.

Version policy: fix/perf and releasable maintenance changes are patch; features
are minor, including below 1.0; breaking changes are major after 1.0 and minor
below 1.0. Breaking classification takes precedence. The adapter explicitly maps
pre-1.0 features onto changelogen's pre-1.0 behavior. Every valid semantic type is
eligible except routine release bookkeeping and internal planning labels. New
public notes and suggested subjects omit internal IDs. Preserve historical notes.
Contributors are rendered by changelogen from Git metadata and an optional reviewed
handle map, with online lookup disabled so repeated preparation is deterministic.

Vue CI runs only on develop pushes, including merge/release commits. Lint, tests,
format and build failures are fatal. PR/release-branch events do not duplicate it.
After validation, one release/update-version PR targets develop; its title and
commit remain ci(release): update version. Use GITHUB_TOKEN for PR API operations
and GH_TOKEN for release-branch pushes. Generated files are validated in that run;
tested-source freshness is checked before preparation and before PR update.
.release/prepared.json records source, baseline, generator, version and notes digest.

The owner reviews/merges preparation, then promotes develop → main with an ordinary
merge commit preserving ancestry (no squash/rebase for promotion). No special
merge title is required. Every main push validates eligibility; a valid unpublished
preparation publishes, an already published version is an explicit successful
no-op, and an invalid/stale pending release fails. Publication and artifacts use
the same exact prepared section extractor. Only CRLF/LF and surrounding blank
newline delimiters are normalized; wording, links and spaces remain significant.
See docs/RELEASING.md for settings, token events and owner verification.

Prefer Rust for new repository-owned validation and migration utilities. The
changelogen adapter is JavaScript because it calls the authoritative npm APIs;
Studio's Vue/TypeScript and existing thin installer/fixture wrappers remain.

Product contract: Studio connects to Kit's loopback HTTP MCP endpoint (default
http://127.0.0.1:47601/mcp), minimum HTTP release 0.6.0. It never discovers,
launches, owns or stops a Kit subprocess. Old stdio settings are disabled and
backed up before replacement; selected tool restrictions remain. Public Studio
→ Kit MCP uses the revision documented in docs/mcp-kit-version.txt. The private
native/WASM plugin ABI is unchanged. Kit/Infer own shared ~/.rune installation,
storage and migration. Never fall back to CWD or overwrite conflicting data.

Implement only Screen Insight for now. Its static workflow registry is in
src-tauri/src/workflow.rs. Capture follows explicit screen/window selection and
a user action; Insight frames remain in memory and never use archive capture
or continuous monitoring. Require an explicit Analysis model with advertised
vision/tool support. Keep external-agent and API sessions separate.