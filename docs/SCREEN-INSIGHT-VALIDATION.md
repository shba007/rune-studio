# Screen Insight and Kit HTTP validation

Validated locally on Windows, 24 October 2026. No commits, pushes, tags,
releases, publishing workflows, or live release PRs were created by the agent.
Existing staged owner work was preserved.

## Changes and files

- `src-tauri/src/workflow.rs`: one static Screen Insight registration, typed input,
  capabilities, timeout/cancellation, generation protection and existing tracing.
- `src-tauri/src/observe/screenshot.rs`: explicit screen/window enumeration and
  one-frame capture without writing an archive image.
- `src-tauri/src/analysis/{run,state}.rs`, analysis question prompt, commands/lib:
  shared Infer inference, redacted call traces, in-memory preview, close/discard
  behaviour, guarded results and workflow commands.
- AnalysisCard, useAnalysis/useAgent, protocol/config types and tests: explicit
  target picker, question, progress/error/result, preview, model and capture time.
- Kit client, header/schema modules, SettingsTools and tests: HTTP/SSE endpoint,
  per-request metadata and header annotations, exact selection, cancellation,
  response bounds, error propagation and legacy settings migration. The obsolete
  standalone stdio gateway fixture and process-termination connection path were removed.
- Three GitHub Actions workflows: restored validated preparation/publication,
  latest-source checks, exact changelog extraction, installer delivery and missing
  Linux development packages. Existing manifest/lock synchronized to package 0.0.26.
- AGENTS, README, ARCHITECTURE, RELEASING, Kit and Screen Insight documentation:
  new transport/ownership contract and future workflow ideas only.

## Actual results

- Application Rust: 498 passed, 12 environment-specific tests ignored in the normal
  suite. Workflow lifecycle, model input/error/cancellation and transport tests passed.
- Frontend: 167 passed across 27 files. Target-selection/preview and endpoint/error
  regression tests also passed in focused reruns.
- Release utility: 9 passed, including deterministic repeat preparation, consumed
  history, historical preservation, version synchronization and exact note extraction.
- Rust Clippy, rustfmt, Vue/TypeScript typecheck, frontend lint/format/build,
  Windows release-profile compilation, workflow YAML parsing and diff checks passed.
- Explicit live test: native capture of primary screen 65537 (1280x720 model frame)
  reached local Infer at 127.0.0.1:8080 using gemma-4-12b-it and returned a nonempty
  report. The capture had an empty archive path; no test image was retained. This
  test used an explicit process-only model override, leaving saved settings unchanged.
- Actual Kit executable: code-kit/target/debug/rune-kit.exe, HTTP preview reporting
  0.5.0. Tested with explicit debug preview opt-in only. Minimum production HTTP
  release remains 0.6.0, as agreed in Kit's MCP-CONTRACT.md.
- Actual plugin artifacts: time WASM and git native executable from code-tools
  (0.3.0 manifests); installed filesystem WASM 0.4.1. Time query, Git commit-message
  validation, file read/list, deselection, outside-root refusal, missing-file errors
  and attempts to replace allowed_dir passed. Filesystem checks passed across a
  controlled restart at the same endpoint. Disposable registry/root data lived in
  src-tauri/target; no installed registry or sibling source was changed.

## Migration and limitations

Old Kit executable settings become disabled, retaining selections. Save preserves
original bytes in kit.stdio-backup.json first. Repeated migration is safe; conflicting
backups or malformed originals refuse replacement. Start Kit separately, configure
server confinement, confirm the endpoint and explicitly re-enable tools. Studio
never terminates a shared Kit process or replays uncertain tool operations.

Use installed Kit plugin names for stable exact tool identities; this local Kit
preview namespaces direct artifact-path tools with the path, which Studio correctly
refuses. This was not bypassed or patched in the sibling repository.

Saved Analysis role was empty and still requires owner configuration. Images remain
in memory; existing traces contain redacted request metadata and results, not image
payloads. No background capture or additional workflows were introduced.

Linux/macOS execution, capture permission prompts, packaged installer installation,
full live Tauri picker interaction and GitHub event/token settings remain untested.
The Linux dependency correction needs the owner's CI run. The pure/UI and real
capture/Infer checks do not imply those platform or publishing checks ran.

## Release dry run and owner steps

Two identical read-only prepare runs produced the same proposal:

```markdown
Proposed version: 0.0.27

### Features

- implement external agent screen request handling and UI updates
```

That is committed-history input, not a generated description of uncommitted work.
The synthetic example yields 0.0.24 with the substantive fix entry documented in
RELEASING. Current 0.0.26 notes extract exactly from the existing changelog; historical
published entries were not rewritten. CI recalculates after the owner commits.

Owner sequence: commit/push develop → successful validation → Actions updates the
single release/update-version PR → owner reviews/merges → promote develop to main
using a merge commit titled `ci(release): publish vX.Y.Z` → main publishes prepared
notes/tag → release.published builds/uploads installers. Merging into develop alone
never publishes. Preparation rechecks source freshness immediately before PR update.

Required settings: Actions may create PRs; job GITHUB_TOKEN grants PR write;
event-capable GH_TOKEN grants Contents write for branch pushes and release creation;
existing Tauri signing secrets remain configured. Values were not read or exposed.
Vue/TypeScript, Bun/Vite, third-party Actions/YAML and existing thin installer/desktop
fixture wrappers remain; repository-owned release generation stays in Rust xtask.

Suggested owner commit subject: `feat: add Screen Insight and Kit HTTP integration`.