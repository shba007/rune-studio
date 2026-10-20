# Changelog

## v0.0.13

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.12...v0.0.13)

### 🚀 Enhancements

- **bridge:** Name the outside agent in control, from its declaration or its registration ([b4dfc69](https://github.com/shba007/rune-studio/commit/b4dfc69))
- **ui:** Show which agent is in control on the orb, its ring and the top bar ([ec546cc](https://github.com/shba007/rune-studio/commit/ec546cc))
- **demo:** Let recorders see Rune while the model never does ([806bb54](https://github.com/shba007/rune-studio/commit/806bb54))
- **trace:** Store reflection runs (schema v4) and list them in the CLI ([be537b9](https://github.com/shba007/rune-studio/commit/be537b9))
- **reflect:** The idle reflection shell with a stub job ([717e6f2](https://github.com/shba007/rune-studio/commit/717e6f2))
- **ui:** The idle review prompt, the reflecting orb and the review progress ring ([5c499b5](https://github.com/shba007/rune-studio/commit/5c499b5))

### 📖 Documentation

- Use cases with features, flow, limits, upgrade scope and measured runs ([70c7d00](https://github.com/shba007/rune-studio/commit/70c7d00))
- One readable page per use case, replacing the single use-cases file ([a148e2d](https://github.com/shba007/rune-studio/commit/a148e2d))
- Use-case pages stand alone, with a numbered How to use in each ([e968f37](https://github.com/shba007/rune-studio/commit/e968f37))
- Drop the obvious first step from the use-case pages ([92f9c62](https://github.com/shba007/rune-studio/commit/92f9c62))
- Add the agent identity report, document it, remove finished handoff files ([9aa1c40](https://github.com/shba007/rune-studio/commit/9aa1c40))
- Handoff R6a, screen analysis as an overlay, and the R6 roadmap entry ([e1f2df8](https://github.com/shba007/rune-studio/commit/e1f2df8))

### ❤️ Contributors

- Claude <noreply@anthropic.com>

## v0.0.12

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.11...v0.0.12)

### 🩹 Fixes

- No deadlock at task start, and a valid tools array for screen analysis ([9748ca1](https://github.com/shba007/rune-studio/commit/9748ca1))
- One lock for the run row, a real tools array test, Windows-safe test paths ([0262641](https://github.com/shba007/rune-studio/commit/0262641))
- **safety:** Give the keyboard back to the target after a confirm ([19b3ec5](https://github.com/shba007/rune-studio/commit/19b3ec5))
- **agent:** Stop calling correct typing and scrolling stuck ([28e5ae9](https://github.com/shba007/rune-studio/commit/28e5ae9))
- **exec:** Open_app keeps local page queries and never opens an extensionless file ([53c571e](https://github.com/shba007/rune-studio/commit/53c571e))
- **fast-checks:** Use the picked probability and the served decision model name ([11ad99e](https://github.com/shba007/rune-studio/commit/11ad99e))
- **voice:** Transcribe only, cap the reply, drop loops, show the words before acting ([4493d92](https://github.com/shba007/rune-studio/commit/4493d92))
- **model:** Repair trailing junk in tool arguments and answer bad ones instead of failing ([1c46974](https://github.com/shba007/rune-studio/commit/1c46974))
- **verify:** Tell the checker which window is in front, as a system fact ([2e51f1e](https://github.com/shba007/rune-studio/commit/2e51f1e))
- **eval:** Do not trust a server token count of 1, estimate and mark it ([71f38b7](https://github.com/shba007/rune-studio/commit/71f38b7))
- **cli:** Say when another program holds the bridge port, exit 1 for an unknown id ([c44d383](https://github.com/shba007/rune-studio/commit/c44d383))
- **analysis:** Hide the unreliable boxes unless analysisBoxes is on, retry the failed question ([ac37a32](https://github.com/shba007/rune-studio/commit/ac37a32))
- **ui:** CamelCase the confirm card and step started payloads ([830b351](https://github.com/shba007/rune-studio/commit/830b351))
- One version for the app, package.json's ([22dc983](https://github.com/shba007/rune-studio/commit/22dc983))
- Keep pixel_at_model for the Windows desktop tests that use it ([f579468](https://github.com/shba007/rune-studio/commit/f579468))
- **analysis:** Capture before the window opens and keep tasks out while one runs ([a3c7786](https://github.com/shba007/rune-studio/commit/a3c7786))

### 💅 Refactors

- Remove dead voice and loop code and clear every clippy warning ([5bf9835](https://github.com/shba007/rune-studio/commit/5bf9835))

### 📖 Documentation

- Handoff F1, fixes from the field test ([d929919](https://github.com/shba007/rune-studio/commit/d929919))
- Handoff R5, agent identity, demo mode and the idle reflection shell ([21d1e2a](https://github.com/shba007/rune-studio/commit/21d1e2a))
- Add the field-test fix report, remove finished handoffs and generated files ([0531bec](https://github.com/shba007/rune-studio/commit/0531bec))

### ✅ Tests

- **model:** A cancelled model call really closes its connection ([24105c8](https://github.com/shba007/rune-studio/commit/24105c8))

### 🎨 Styles

- Cargo fmt ([f95d9a1](https://github.com/shba007/rune-studio/commit/f95d9a1))

### ❤️ Contributors

- Claude <noreply@anthropic.com>
- Shirsendu Bairagi ([@shba007](https://github.com/shba007))

## v0.0.11

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.10...v0.0.11)

### 🚀 Enhancements

- **analysis:** Ask the model for an annotated report ([3fac51f](https://github.com/shba007/rune-studio/commit/3fac51f))
- **trace:** Store visual analyses (schema v3) ([1775b53](https://github.com/shba007/rune-studio/commit/1775b53))
- **analysis:** Capture, ask and show in its own window ([d9ec139](https://github.com/shba007/rune-studio/commit/d9ec139))
- **ui:** The screen analysis window and its pill button ([3555518](https://github.com/shba007/rune-studio/commit/3555518))

### 💅 Refactors

- **analysis:** Express the status order as matches! ([781a5cc](https://github.com/shba007/rune-studio/commit/781a5cc))

### 📖 Documentation

- Remove the handoff for the completed R3 milestone ([b2e3691](https://github.com/shba007/rune-studio/commit/b2e3691))
- Field test guide for milestones R0 to R4 ([be1db1e](https://github.com/shba007/rune-studio/commit/be1db1e))
- Take the field test guide out of the repository ([0863638](https://github.com/shba007/rune-studio/commit/0863638))
- Visual analysis (architecture, readme, roadmap) and the R4 report ([fab608a](https://github.com/shba007/rune-studio/commit/fab608a))

### 🏡 Chore

- Apply code fixes [skip ci] ([54ae37a](https://github.com/shba007/rune-studio/commit/54ae37a))

### ✅ Tests

- **probe:** Measure bounding boxes, not just points ([c68c681](https://github.com/shba007/rune-studio/commit/c68c681))

### ❤️ Contributors

- Claude <noreply@anthropic.com>
- Shba007

## v0.0.10

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.9...v0.0.10)

### 🚀 Enhancements

- **trace:** Store the independent check of a run (schema version 2) ([4109a45](https://github.com/shba007/rune-studio/commit/4109a45))
- **verify:** The independent check as a forced verdict tool call ([e3fef4c](https://github.com/shba007/rune-studio/commit/e3fef4c))
- **verify:** Run the check after a task, or before it finishes ([595b982](https://github.com/shba007/rune-studio/commit/595b982))
- **eval:** Measure the verifier against the machine's checkers ([cce0401](https://github.com/shba007/rune-studio/commit/cce0401))

### 📖 Documentation

- Remove the handoff for the completed R2 milestone ([65508b4](https://github.com/shba007/rune-studio/commit/65508b4))
- Handoff for R4, visual analysis ([cafe338](https://github.com/shba007/rune-studio/commit/cafe338))
- R4 handoff works directly on develop ([6451972](https://github.com/shba007/rune-studio/commit/6451972))
- Describe the verifier and how to measure it ([d3e67ad](https://github.com/shba007/rune-studio/commit/d3e67ad))
- Add the verifier report ([e7d0efc](https://github.com/shba007/rune-studio/commit/e7d0efc))
- Correct the commit hashes in the verifier report ([0bb71f5](https://github.com/shba007/rune-studio/commit/0bb71f5))

### ❤️ Contributors

- Claude <noreply@anthropic.com>

## v0.0.9

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.8...v0.0.9)

### 🚀 Enhancements

- **eval:** The suite format, its hard limits and the checkers ([32097cd](https://github.com/shba007/rune-studio/commit/32097cd))
- **eval:** The 15-task suite, its fixture pages and instructions ([f4661de](https://github.com/shba007/rune-studio/commit/f4661de))
- **eval:** The eval mode that runs the suite through the built-in agent ([11bec84](https://github.com/shba007/rune-studio/commit/11bec84))
- **eval:** The runner script and the report with the success and speed gate ([2e092cd](https://github.com/shba007/rune-studio/commit/2e092cd))

### 📖 Documentation

- Remove the handoffs for completed milestones ([2e3cea5](https://github.com/shba007/rune-studio/commit/2e3cea5))
- Describe the evaluation harness and how to commit a baseline ([bb23b0a](https://github.com/shba007/rune-studio/commit/bb23b0a))
- Add the evaluation harness report ([6152429](https://github.com/shba007/rune-studio/commit/6152429))

### ❤️ Contributors

- Claude <noreply@anthropic.com>

## v0.0.8

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.7...v0.0.8)

### 🚀 Enhancements

- **archive:** One folder per session with a manifest, so a task's evidence is in one place ([0d6843a](https://github.com/shba007/rune-studio/commit/0d6843a))
- **state:** One is_running answer and an AgentState::Reflecting ([3d4f954](https://github.com/shba007/rune-studio/commit/3d4f954))
- **model:** Retry transient chat errors up to three times ([7e948f7](https://github.com/shba007/rune-studio/commit/7e948f7))
- **model:** Read both /v1/decisions answer shapes; add scripts/wire-audit.mjs ([5c5cfbc](https://github.com/shba007/rune-studio/commit/5c5cfbc))
- Single instance; a second launch opens the existing pill ([9cef8b3](https://github.com/shba007/rune-studio/commit/9cef8b3))
- **safety:** Notice a front window that runs as administrator ([fd36a66](https://github.com/shba007/rune-studio/commit/fd36a66))
- Foundations (R0): per-run cancellation, panic supervisor, retries, CI ([c0350b3](https://github.com/shba007/rune-studio/commit/c0350b3))
- **model:** Log what the decision model answers, with the picked option's probability ([b48e973](https://github.com/shba007/rune-studio/commit/b48e973))
- **backend:** Read the model's capabilities and warn when it lists no Audio ([8f0aa04](https://github.com/shba007/rune-studio/commit/8f0aa04))
- **trace:** Add bundled SQLite and stop what a finished run leaves behind ([fd41c53](https://github.com/shba007/rune-studio/commit/fd41c53))
- **trace:** Add the trace store and its writer thread ([d768a45](https://github.com/shba007/rune-studio/commit/d768a45))
- **observe:** Keep the path of the stored screenshot in Frame ([5959d4c](https://github.com/shba007/rune-studio/commit/5959d4c))
- **trace:** Record the built-in agent's runs ([6ba77cc](https://github.com/shba007/rune-studio/commit/6ba77cc))
- **trace:** Record outside agents' runs ([1bd44b5](https://github.com/shba007/rune-studio/commit/1bd44b5))
- **cli:** List and show recorded runs ([21514f2](https://github.com/shba007/rune-studio/commit/21514f2))
- **archive:** Opt-in retention of old session folders ([c8317b2](https://github.com/shba007/rune-studio/commit/c8317b2))
- **trace): flight recorder (R1:** SQLite record of every run ([5083a12](https://github.com/shba007/rune-studio/commit/5083a12))

### 🩹 Fixes

- **exec:** Give every run its own cancellation scope ([65dec10](https://github.com/shba007/rune-studio/commit/65dec10))
- **agent:** A panic inside a run still ends the task ([842b992](https://github.com/shba007/rune-studio/commit/842b992))
- **bridge:** Outside-agent actions take turns ([6d40588](https://github.com/shba007/rune-studio/commit/6d40588))
- A poisoned lock can no longer take Stop down ([9634c63](https://github.com/shba007/rune-studio/commit/9634c63))
- **model:** Send audio as input_audio and the thinking switch rune-infer honours ([23ba597](https://github.com/shba007/rune-studio/commit/23ba597))
- **model:** Align requests with rune-infer (input_audio, thinking object) ([353a1a9](https://github.com/shba007/rune-studio/commit/353a1a9))
- **trace:** Clear the clippy findings in the recorder code ([b703baa](https://github.com/shba007/rune-studio/commit/b703baa))

### 📖 Documentation

- Rewrite README and architecture doc from the code, fix CLI names in the agent guide ([8d1a928](https://github.com/shba007/rune-studio/commit/8d1a928))
- Add roadmap for the reflective agent, ecosystem section, wire-format findings ([32b63db](https://github.com/shba007/rune-studio/commit/32b63db))
- Record rune-infer Postman spec findings (audio, thinking, decisions, image endpoints) ([f93bf6b](https://github.com/shba007/rune-studio/commit/f93bf6b))
- Handoff for milestone R0 (foundations) ([543e6f4](https://github.com/shba007/rune-studio/commit/543e6f4))
- Describe the R0 foundations as built ([d800336](https://github.com/shba007/rune-studio/commit/d800336))
- Add the foundations handoff report ([fa675bf](https://github.com/shba007/rune-studio/commit/fa675bf))
- Handoff for merging the orchestration branch into develop ([95ad0e3](https://github.com/shba007/rune-studio/commit/95ad0e3))
- Handoff for R0.5, align the chat request with what rune-infer accepts ([7411f42](https://github.com/shba007/rune-studio/commit/7411f42))
- Handoff for R1, the flight recorder ([009f15d](https://github.com/shba007/rune-studio/commit/009f15d))
- Audio is input_audio, thinking is the thinking object, decision shape confirmed ([16e663c](https://github.com/shba007/rune-studio/commit/16e663c))
- Add the wire alignment report ([35485c3](https://github.com/shba007/rune-studio/commit/35485c3))
- Handoff for R2, the evaluation harness ([618c054](https://github.com/shba007/rune-studio/commit/618c054))
- Describe the trace store, its CLI commands and retention ([c5871ea](https://github.com/shba007/rune-studio/commit/c5871ea))
- Add the flight recorder report ([91844e8](https://github.com/shba007/rune-studio/commit/91844e8))
- Fix the commit count in the flight recorder report ([b2a08dc](https://github.com/shba007/rune-studio/commit/b2a08dc))
- Handoff for R3, the verifier ([96dccf1](https://github.com/shba007/rune-studio/commit/96dccf1))
- Update description of the desktop agent to 'harness' ([5e8c5a0](https://github.com/shba007/rune-studio/commit/5e8c5a0))

### 🤖 CI

- Run the Rust tests on Windows and Ubuntu ([c51286e](https://github.com/shba007/rune-studio/commit/c51286e))
- Remove the Rust test workflow ([d9ae02d](https://github.com/shba007/rune-studio/commit/d9ae02d))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))
- Claude <noreply@anthropic.com>

## v0.0.7

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.6...v0.0.7)

### 🤖 CI

- **deploy:** Stop adding the CLI to the bundles by hand, Tauri already bundles every binary ([#13](https://github.com/shba007/rune-studio/pull/13))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))

## v0.0.6

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.5...v0.0.6)

### 🚀 Enhancements

- **installer:** Add the CLI folder to the user's PATH ([15468e6](https://github.com/shba007/rune-studio/commit/15468e6))
- Ship the CLI with the app on every OS, put it on PATH, fix settings scrolling ([3be4704](https://github.com/shba007/rune-studio/commit/3be4704))

### 🩹 Fixes

- **settings:** Scroll inside the settings page; feat: CLI on PATH and in the bundles on macOS and Linux ([f92cc56](https://github.com/shba007/rune-studio/commit/f92cc56))

### 🤖 CI

- **deploy:** Rename artifact steps and ship the CLI inside the Windows installer ([4a78750](https://github.com/shba007/rune-studio/commit/4a78750))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))
- Claude <noreply@anthropic.com>

## v0.0.5

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.4...v0.0.5)

### 🚀 Enhancements

- **updater:** Check at start-up, prompt, install with orb progress and restart; version and manual check in settings ([936fd88](https://github.com/shba007/rune-studio/commit/936fd88))
- **updater:** In-app update check, prompt, orb progress and auto restart ([35f8a6b](https://github.com/shba007/rune-studio/commit/35f8a6b))

### 🩹 Fixes

- Add missing newlines at the end of files for consistency ([71f661b](https://github.com/shba007/rune-studio/commit/71f661b))

### ✅ Tests

- **updater:** Type the mock functions to satisfy lint ([85cba67](https://github.com/shba007/rune-studio/commit/85cba67))
- **updater:** Type the mock functions to satisfy lint ([083198c](https://github.com/shba007/rune-studio/commit/083198c))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))
- Claude <noreply@anthropic.com>

## v0.0.4

### 🚀 Enhancements

- One omni model, direct voice, cursor to centre, and a fail-closed record folder ([d11d739](https://github.com/shba007/rune-studio/commit/d11d739))

### 🏡 Chore

- Apply code fixes [skip ci] ([143be95](https://github.com/shba007/rune-studio/commit/143be95))

### ❤️ Contributors

- Shba007 ([@shba007](https://github.com/shba007))
- Shirsendu Bairagi ([@shba007](https://github.com/shba007))

## v0.0.3

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.2...v0.0.3)

### 🏡 Chore

- Comment out mobile build steps in deploy workflow ([6ca8684](https://github.com/shba007/rune-studio/commit/6ca8684))

### 🤖 CI

- **deploy:** Install pipewire and other native deps for the Linux desktop build ([3ad334b](https://github.com/shba007/rune-studio/commit/3ad334b))
- **deploy:** Install native deps for the Linux desktop build ([62e4a36](https://github.com/shba007/rune-studio/commit/62e4a36))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))
- Claude <noreply@anthropic.com>

## v0.0.2

[compare changes](https://github.com/shba007/rune-studio/compare/v0.0.1...v0.0.2)

### 🚀 Enhancements

- **build:** Rename binaries to rune-studio (app) and rune-studio-cli, publish the CLI ([88331ab](https://github.com/shba007/rune-studio/commit/88331ab))
- **build:** Rename binaries and publish rune-studio-cli in the release ([b65db6e](https://github.com/shba007/rune-studio/commit/b65db6e))

### 🩹 Fixes

- Update GITHUB_TOKEN secret reference in release workflow ([1fa6978](https://github.com/shba007/rune-studio/commit/1fa6978))

### 🏡 Chore

- Update dependencies in package.json, Cargo.toml, and Cargo.lock ([4df72f6](https://github.com/shba007/rune-studio/commit/4df72f6))

### 🤖 CI

- **deploy:** Disable build-web job and init android project before signing ([740fc13](https://github.com/shba007/rune-studio/commit/740fc13))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))
- Claude <noreply@anthropic.com>

## v0.0.1

### 🚀 Enhancements

- Project init add new components for environment lighting, PC monitor, printer, surface, and table ([6c8e8ed](https://github.com/shba007/rune-studio/commit/6c8e8ed))
- Add media scanning functionality and media comparison features ([607f791](https://github.com/shba007/rune-studio/commit/607f791))
- Enhance mediaComparatorview with auto-scan and artifact preloading ([91d1d25](https://github.com/shba007/rune-studio/commit/91d1d25))
- Restart as Rune Studio desktop agent, add M0 pill shell ([e0ee984](https://github.com/shba007/rune-studio/commit/e0ee984))
- Dot-matrix orb glyph with ambient/open/settings pill modes ([65c9fff](https://github.com/shba007/rune-studio/commit/65c9fff))
- Add composer card, bottom-right anchoring and live mic level ([34e669b](https://github.com/shba007/rune-studio/commit/34e669b))
- Redesign pill UI, adopt Oxanium and Phosphor Fill icon system ([6a4a058](https://github.com/shba007/rune-studio/commit/6a4a058))
- Add M1 eyes and hands (capture, coordinate mapping, input, Stop) ([e0cf7d9](https://github.com/shba007/rune-studio/commit/e0cf7d9))
- Add control overlay, snap UI to 8px grid, add M1.5 grounding probe ([539e7a6](https://github.com/shba007/rune-studio/commit/539e7a6))
- Add M2 agent loop (model client, action protocol, open_app, start_task) ([fd4359d](https://github.com/shba007/rune-studio/commit/fd4359d))
- Show agent actions on screen, merge Stop into the send button ([4fbbd47](https://github.com/shba007/rune-studio/commit/4fbbd47))
- Add external agent bridge (M2.5) behind the agent-bridge feature ([bcaa066](https://github.com/shba007/rune-studio/commit/bcaa066))
- Add rune-mcp launcher so an agent can start Rune Studio without installing ([6145c2e](https://github.com/shba007/rune-studio/commit/6145c2e))
- Add the rune-studio command and rewrite the agent guide for end users ([31512b5](https://github.com/shba007/rune-studio/commit/31512b5))
- Glide the cursor smoothly and add a typing effect to the control overlay ([3d4e8fb](https://github.com/shba007/rune-studio/commit/3d4e8fb))
- Package the agent guide as a Claude Code skill and as MCP resources and prompts ([e733d65](https://github.com/shba007/rune-studio/commit/e733d65))
- New logo drawn from the dot-matrix orb, across icons, favicon, README and Settings ([d315c4f](https://github.com/shba007/rune-studio/commit/d315c4f))
- Connect and disconnect Antigravity; clear message for unknown agents ([fd3a0c8](https://github.com/shba007/rune-studio/commit/fd3a0c8))
- One-pill control overlay with stop icon, semantic tool icons and a screenshot flash ([53d7c04](https://github.com/shba007/rune-studio/commit/53d7c04))
- Vue devtools in development, calmer screenshot effect, typing ripple at the caret ([c6d0da2](https://github.com/shba007/rune-studio/commit/c6d0da2))
- One-row composer the height of the orb, health badge on the orb, no status row ([e5e3903](https://github.com/shba007/rune-studio/commit/e5e3903))
- Dark composer in dark mode, health shown as three dots on the orb instead of a green badge ([eee1571](https://github.com/shba007/rune-studio/commit/eee1571))
- Add the safety gate, risky actions ask the user first ([62b698a](https://github.com/shba007/rune-studio/commit/62b698a))
- Tick dial and dot wave effect, faint hex reticle at the focus, recognition api spec ([6808997](https://github.com/shba007/rune-studio/commit/6808997))
- Remove the hex hud, large ring arcs again, dot-matrix pointer while the agent controls ([ca408c1](https://github.com/shba007/rune-studio/commit/ca408c1))
- Fast checks with the decision model, skip planning, spot a finished task, second risk opinion ([38ca43b](https://github.com/shba007/rune-studio/commit/38ca43b))
- Precision control, element ids, scroll, drag, ask_user, stuck detection ([f9f1242](https://github.com/shba007/rune-studio/commit/f9f1242))
- Screen reading with the decision model, wait for loading pages, hints for pop-ups and logins ([9dbf349](https://github.com/shba007/rune-studio/commit/9dbf349))
- M5 voice, speech detection, transcription, spoken tasks, steering, stop and answers ([c00a4e6](https://github.com/shba007/rune-studio/commit/c00a4e6))

### 🩹 Fixes

- Running `rune-studio enable` again re-arms an agent that Stop locked out ([6d76114](https://github.com/shba007/rune-studio/commit/6d76114))
- Type at 15 ms per character so Notepad no longer drops or repeats letters ([d0b0f6b](https://github.com/shba007/rune-studio/commit/d0b0f6b))
- Click a control where it is now, not where it was when the list was made ([5654cdd](https://github.com/shba007/rune-studio/commit/5654cdd))

### 💅 Refactors

- Remove starter commands from suggestions configuration ([74de631](https://github.com/shba007/rune-studio/commit/74de631))

### 📖 Documentation

- Record M1.5 grounding results and decision ([e92d242](https://github.com/shba007/rune-studio/commit/e92d242))
- Restore docs/SETUP.md and describe the skill, disconnect and MCP guide ([6362fa4](https://github.com/shba007/rune-studio/commit/6362fa4))
- Document the skill, disconnect and the MCP guide in setup and architecture ([da141f7](https://github.com/shba007/rune-studio/commit/da141f7))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))

## v0.0.1

### 🚀 Enhancements

- Project init add new components for environment lighting, PC monitor, printer, surface, and table ([6c8e8ed](https://github.com/shba007/rune-studio/commit/6c8e8ed))
- Add media scanning functionality and media comparison features ([607f791](https://github.com/shba007/rune-studio/commit/607f791))
- Enhance mediaComparatorview with auto-scan and artifact preloading ([91d1d25](https://github.com/shba007/rune-studio/commit/91d1d25))
- Restart as Rune Studio desktop agent, add M0 pill shell ([e0ee984](https://github.com/shba007/rune-studio/commit/e0ee984))
- Dot-matrix orb glyph with ambient/open/settings pill modes ([65c9fff](https://github.com/shba007/rune-studio/commit/65c9fff))
- Add composer card, bottom-right anchoring and live mic level ([34e669b](https://github.com/shba007/rune-studio/commit/34e669b))
- Redesign pill UI, adopt Oxanium and Phosphor Fill icon system ([6a4a058](https://github.com/shba007/rune-studio/commit/6a4a058))
- Add M1 eyes and hands (capture, coordinate mapping, input, Stop) ([e0cf7d9](https://github.com/shba007/rune-studio/commit/e0cf7d9))
- Add control overlay, snap UI to 8px grid, add M1.5 grounding probe ([539e7a6](https://github.com/shba007/rune-studio/commit/539e7a6))
- Add M2 agent loop (model client, action protocol, open_app, start_task) ([fd4359d](https://github.com/shba007/rune-studio/commit/fd4359d))
- Show agent actions on screen, merge Stop into the send button ([4fbbd47](https://github.com/shba007/rune-studio/commit/4fbbd47))
- Add external agent bridge (M2.5) behind the agent-bridge feature ([bcaa066](https://github.com/shba007/rune-studio/commit/bcaa066))
- Add rune-mcp launcher so an agent can start Rune Studio without installing ([6145c2e](https://github.com/shba007/rune-studio/commit/6145c2e))
- Add the rune-studio command and rewrite the agent guide for end users ([31512b5](https://github.com/shba007/rune-studio/commit/31512b5))
- Glide the cursor smoothly and add a typing effect to the control overlay ([3d4e8fb](https://github.com/shba007/rune-studio/commit/3d4e8fb))
- Package the agent guide as a Claude Code skill and as MCP resources and prompts ([e733d65](https://github.com/shba007/rune-studio/commit/e733d65))
- New logo drawn from the dot-matrix orb, across icons, favicon, README and Settings ([d315c4f](https://github.com/shba007/rune-studio/commit/d315c4f))
- Connect and disconnect Antigravity; clear message for unknown agents ([fd3a0c8](https://github.com/shba007/rune-studio/commit/fd3a0c8))
- One-pill control overlay with stop icon, semantic tool icons and a screenshot flash ([53d7c04](https://github.com/shba007/rune-studio/commit/53d7c04))
- Vue devtools in development, calmer screenshot effect, typing ripple at the caret ([c6d0da2](https://github.com/shba007/rune-studio/commit/c6d0da2))
- One-row composer the height of the orb, health badge on the orb, no status row ([e5e3903](https://github.com/shba007/rune-studio/commit/e5e3903))
- Dark composer in dark mode, health shown as three dots on the orb instead of a green badge ([eee1571](https://github.com/shba007/rune-studio/commit/eee1571))
- Add the safety gate, risky actions ask the user first ([62b698a](https://github.com/shba007/rune-studio/commit/62b698a))
- Tick dial and dot wave effect, faint hex reticle at the focus, recognition api spec ([6808997](https://github.com/shba007/rune-studio/commit/6808997))
- Remove the hex hud, large ring arcs again, dot-matrix pointer while the agent controls ([ca408c1](https://github.com/shba007/rune-studio/commit/ca408c1))
- Fast checks with the decision model, skip planning, spot a finished task, second risk opinion ([38ca43b](https://github.com/shba007/rune-studio/commit/38ca43b))
- Precision control, element ids, scroll, drag, ask_user, stuck detection ([f9f1242](https://github.com/shba007/rune-studio/commit/f9f1242))
- Screen reading with the decision model, wait for loading pages, hints for pop-ups and logins ([9dbf349](https://github.com/shba007/rune-studio/commit/9dbf349))
- M5 voice, speech detection, transcription, spoken tasks, steering, stop and answers ([c00a4e6](https://github.com/shba007/rune-studio/commit/c00a4e6))

### 🩹 Fixes

- Running `rune-studio enable` again re-arms an agent that Stop locked out ([6d76114](https://github.com/shba007/rune-studio/commit/6d76114))
- Type at 15 ms per character so Notepad no longer drops or repeats letters ([d0b0f6b](https://github.com/shba007/rune-studio/commit/d0b0f6b))
- Click a control where it is now, not where it was when the list was made ([5654cdd](https://github.com/shba007/rune-studio/commit/5654cdd))

### 💅 Refactors

- Remove starter commands from suggestions configuration ([74de631](https://github.com/shba007/rune-studio/commit/74de631))

### 📖 Documentation

- Record M1.5 grounding results and decision ([e92d242](https://github.com/shba007/rune-studio/commit/e92d242))
- Restore docs/SETUP.md and describe the skill, disconnect and MCP guide ([6362fa4](https://github.com/shba007/rune-studio/commit/6362fa4))
- Document the skill, disconnect and the MCP guide in setup and architecture ([da141f7](https://github.com/shba007/rune-studio/commit/da141f7))

### ❤️ Contributors

- Shirsendu Bairagi ([@shba007](https://github.com/shba007))