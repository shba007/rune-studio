# Changelog

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