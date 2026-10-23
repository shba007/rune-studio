# Working agreement

Implement and test locally. Leave changes uncommitted and unstaged. The owner
performs commits, pushes, merges, tags and releases. Do not dispatch publishing
workflows. Preserve unrelated work and do not modify sibling repositories.

Prepare package.json version and its substantive CHANGELOG.md section before
the owner commits. New public notes and suggested commit subjects describe
functionality, fixes, compatibility and migration without internal planning IDs.
Do not rewrite historical commits, tags, changelog entries or published releases.

Preview with `node scripts/release-notes.mjs`; test with
`node --test scripts/release-notes.test.mjs`. Release automation must extract this
exact version section, never regenerate it from Git history. CI must not create
source commits or push branches. After the owner's main-branch release action
(commit message containing `ci(release):`), release.yml publishes the prepared
tag/release and deploy.yml builds/uploads installers. No agent runs that action.

Kit discovery is explicit executable, then actual user home `rune/rune-kit[.exe]`,
then PATH `rune-kit[.exe]`. No legacy `rune` fallback. Studio is an Infer HTTP
client; it does not install or manage Infer processes. Shared product installers
and storage migration belong to Kit/Infer. Never fall back to the working directory
if home resolution fails, overwrite conflicting data, or move model/project files.

Public Studio → Kit MCP is 2026-07-28 stdio, requiring Kit 0.5.0+. The native/WASM
plugin ABI is internal to Kit/Tools and must not be rewritten as public MCP.