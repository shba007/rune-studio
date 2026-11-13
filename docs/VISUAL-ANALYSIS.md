# Visual Analysis

Select API mode, configure an explicit Analysis role model in the Infer catalog,
then click Visual Analysis. Choose a screen/window and optionally enter a question;
press Ask to capture one frame. Default question: "Summarize what is visible and identify."

Infer must advertise image input and tool calling for that model. The same
captured frame is sent to the configured model and shown with the answer, capture
time and actual model identity. Findings are observations, not verified facts;
image instructions never authorize actions. This workflow exposes no action or
Kit tools to the model. Kit is not needed for Visual Analysis.

Frames stay in memory, are cleared on close, and are not added to screenshot
archives or video. There is no background/continuous capture or change watcher.
A new run cancels/supersedes the previous run. Cancel and Stop discard late model
or capture results. Missing permissions/targets, invalid model capabilities,
Infer errors/busy responses, and timeouts remain visible errors.

The small typed static registry in `src-tauri/src/workflow.rs` supplies identity,
validation, capabilities and execution; its runner owns timeout/cancellation and
presentation lifecycle. Adding a future workflow requires implementing the
interface and static registration, not changing the runner. Only Visual Analysis
is registered. Later candidates (not implemented): selected text explanation,
clipboard summarization, and explicit on-demand document inspection. No designer,
DAG engine, scheduler, marketplace or remote multi-user platform is planned here.

Windows is locally validated. macOS/Linux capture permissions and window-capture
backend support require owner platform tests. Minimized/closed windows fail
rather than silently capturing another target. Linux builds need PipeWire/SPA,
Clang, WebKit/GTK, GBM/EGL, XCB, libxdo and ALSA development packages.

## Playground

Playground opens a separate window for testing supported operations and flows. Visual Analysis uses the configured Infer roles for its planner and optional visual operations.

- The Analysis model is configured in Infer and must advertise image input and tool calling. Image resolution can be chosen for an individual run.
- Sources are Screen, Window and File. A File is re-encoded to PNG by the window and sent as an in-memory image. It is never written to disk or archived, and it never draws the on-screen overlay (its frame uses monitor id `u32::MAX`). Importing a file goes beyond the screen/window capture contract and is an owner-approved exception.
- History is the current session only, held in memory.
- The window is not always-on-top and is hidden from capture.

---

## Validation History & Baselines

Validated locally on Windows, 24 October 2026. No commits, pushes, tags,
releases, publishing workflows, or live release PRs were created by the agent.
Existing staged owner work was preserved.

## Changes and files

- `src-tauri/src/workflow.rs`: one static Visual Analysis registration, typed input,
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
- AGENTS, README, ARCHITECTURE, RELEASING, Kit and Visual Analysis documentation:
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

Suggested owner commit subject: `feat: add Visual Analysis and Kit HTTP integration`.