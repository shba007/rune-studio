# Working agreement

AI agents implement/test locally and leave changes unstaged and uncommitted.
Agents must not commit, push, merge, tag, publish or dispatch publishing workflows.
Preserve unrelated work; do not modify sibling repositories.

GitHub Actions IS authorized to bump the version and changelog, commit/push the
dedicated release/update-version branch, open/update the release PR and publish
tags/releases/artifacts after owner promotion. The human owner commits/pushes
development work and reviews/merges release PRs. Do not use AI services in
release generation.

Releases follow the vuetemplate pipeline. changelogen (`bunx changelogen@latest`)
reads conventional commits, bumps package.json (tauri.conf.json takes its version
from there) and writes CHANGELOG.md. Do not directly author, edit or patch
CHANGELOG.md: CI generates it from reviewed commits.

- integrate.yml (develop pushes): build, lint, format, then open/update the
  `ci(release): update version` PR into develop.
- release.yml (main pushes whose commit says `ci(release):`): create the GitHub
  release and its notes.
- deploy.yml (release published): build desktop installers (Windows, macOS,
  Linux) and the Android APK/AAB and attach them to the release.

The owner merges the release PR into develop, then promotes develop to main.

Product contract: Studio connects to Kit's loopback HTTP MCP endpoint (default
http://127.0.0.1:47601/mcp), minimum HTTP release 0.6.0. Builds with the
embedded-kit feature (Windows release, CI, tauri:dev) run Kit in-process at
launch like embedded Infer, falling back to a free loopback port, and stop it on
exit; onboarding starts it before installing the starter MCP. Studio still never
discovers or spawns an external Kit subprocess, and without the feature it only
connects to a Kit the user runs. Incompatible or old stdio Kit settings are
rejected and the existing file is preserved unchanged; there is no migration or
backup. Selected tool restrictions remain. Public Studio
→ Kit MCP uses the revision documented in docs/mcp-kit-version.txt. The private
native/WASM plugin ABI is unchanged. Kit/Infer own shared ~/.rune installation,
storage and migration. Never fall back to CWD or overwrite conflicting data.

Goal.md holds the current goals and non-goals; read it before proposing work. It
states intent, not status; do not treat it as built. In short: Studio is the
runtime, orchestrator and interface of a local, Jarvis-like harness. A step is a
model call (owned by Infer), a tool call (owned by Kit) or a screen action
(Studio, last resort). Each step shows what it produced, rendered automatically
from the schema Kit or Infer publishes; no model generates a layout. Studio
chooses the model per step and talks only to its embedded Infer, which decides
where the model runs. Lake owns memory; rune-agent owns sub-agents and workflows.
The Playground is the test bench for models, tools and flows in isolation.
When a request is missing something or contradicts itself, Rune asks before
planning instead of assuming. An approved plan may gain steps during a run, and
they are shown as added. What is gathered or produced for a goal is shared across
steps with its origin recorded. Feedback on part of a result redoes only what
depends on that part and yields a new draft.

Not goals for Studio: running models or the MCP servers the harness uses, owning
memory, model-generated layouts, continuous screen watching, direct connections
to remote model servers, macOS. A non-goal is not a removal order: never remove,
disable or rewrite existing code because it is a non-goal without the owner's
explicit permission.

Computer use is the main experience implemented today. The first onboarding
cycle should lead to a working, small computer-use task. Visual Analysis
(formerly Screen Insight; saved records keep the `screen-insight` source id) is a
supporting feature. Its static workflow registry is in src-tauri/src/workflow.rs.
Visual Analysis capture follows explicit screen/window selection and
a user action; its frames remain in memory and never use archive capture
or continuous monitoring. Require an explicit Analysis model with advertised
vision/tool support. Keep external-agent and API sessions separate.