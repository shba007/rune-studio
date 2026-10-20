# R6b report: session video, review prompt, Flight Player

Worked directly on `develop`, one commit per work package, pushed after each. **Nothing here was seen on a real screen**: there is no Windows desktop, no model server, and the pill, overlay and player are capture-excluded. Every behaviour has unit or component tests unless a row says "live check". The one real browser run was headless Chromium against the dev server with fixtures (details under WP5).

## Test numbers

| Command                                         | Before (R6a) | After |
| ----------------------------------------------- | ------------ | ----- |
| `cargo test --lib`                              | 277          | 314   |
| `cargo test --lib --features agent-bridge`      | 318          | 355   |
| `cargo test --lib --features eval,agent-bridge` | 351          | 388   |
| `bun run test:unit` (vitest)                    | 52           | 127   |
| `node --test` (eval + m15)                      | 21           | 21    |

`cargo clippy --all-targets --features agent-bridge` is clean; `cargo check --lib --tests --target x86_64-pc-windows-gnu --features eval,agent-bridge` is clean; `vp lint .` and `vp build` pass (the bundle is 286 kB, 96 kB gzipped, Video.js included).

## Video.js: version and Vue wiring

- Package `@videojs/html`, pinned to **10.0.1** (released 2026-10-02, 18 days old). 10.1.0 (2026-10-13) was newer than two weeks and was not used. It is v10 stable (the "beta" in the handoff is out of date: `latest` is 10.1.0).
- There is no Vue package: Vue uses the HTML custom elements, as the guide shipped inside the package says (`node_modules/@videojs/html/docs/guides/vue.md`).
- `vite.config.ts` passes `compilerOptions.isCustomElement` an exact set, `video-player` and `media-container`, no wildcard.
- `FlightStage.vue` imports `@videojs/html/video/player` and `@videojs/html/ui/container` and renders `<video-player><media-container><video/></media-container></video-player>`. **No packaged skin**: the controls (play, step, speed, details, full screen) are the app's own and call the player's store (`play`, `pause`, `seek`, `setPlaybackRate`), with a fallback to the `<video>` element. The playhead is read from the `<video>` on animation frames while playing.
- The handoff asked for a custom skin with a dot-matrix scrubber: the **timeline is that scrubber** (dots by phase), so the player has no second progress bar.
- Verified in headless Chromium: the player upgrades, plays, seeks and reports time. One library warning (`controlsFeature requires a container element`) went away once `media-container` was added.

## Encoder spike

- The Windows encoder is the Media Foundation sink writer (H.264 from RGB32 frames, Windows-only code in `video/encode.rs`). It type-checks for `x86_64-pc-windows-gnu`. **It has never run**, so there is no real file size per minute and no proof that frames come out right. It can be missing on "N" editions of Windows without the Media Feature Pack; then the run is marked `video.unavailable` and the player shows the screenshots.
- Only a stand-in was measured: a 55 s, 6-frame storyboard encoded with libx264 (ffmpeg, 2 fps, 1280x720) is 17 KB, about 19 KB per minute. A real Media Foundation encode will differ (rate control, key frames, a different encoder). The storyboard schedule (`video/schedule.rs`) is pure and tested.
- Nothing is bundled and no binary was added.

## Work packages

**WP1: schema v6, redaction, counters** (`feat(trace)`). The `calls` table with an index, one row per attempt (outcome `ok`, `retried`, `failed`, `timeout`, `cancelled`, the backoff before it, status, redacted bodies capped at 64 KB, token counts). `runs.step_count`, `retry_count`, `duration_ms` are filled when a run ends. `ChatClient` reports through an `AttemptSink`; cancellation is reported by a drop guard. Wired for plan, decide, verify (blocking and background) and analysis. Tests: v5 to v6 migration, two attempts for a 503 then 200, `cancelled`, redaction (data URLs, audio, key names), truncation, retention.

**WP2a: storyboard video** (`feat(video)`). Pure schedule, even-sized scaling (max 1280 wide), idle-gated worker (a new task cancels it), `session.json` `video` entry. **WP2b: opt-in motion** (`recordSessionVideo: "motion"`, default `off`): a thread captures at 2 fps into `<session>/motion/`. Its cost on a task (CPU, disk, task speed) is **unmeasured**.

**WP3: review prompt** (`feat(review)`). `review.rs` watcher, `ReviewCard.vue` (task, outcome chip, `0:55 · 6 STEPS`, Dismiss and Review, 15 dots), priority confirm > reflection > review, hidden while a task runs, `offerReview` (`always`, `failures`, `never`). Tests for the rules, the card and the timer.

**WP4: read-only data access** (`feat(flight)`). `flight/` with `runs`, `header`, `steps`, `events`, `calls` (no bodies), `call_bodies`, `session_dir`; commands `flight_runs`, `flight_run`, `flight_steps`, `flight_events`, `flight_calls`, `flight_call_body`, `flight_file`, `flight_reveal`. `flight_file` rejects absolute paths, `..` and links that leave the session folder (tests, the link test is Unix-only) and only then adds the one file to the asset-protocol scope. `rune-studio-cli run <id> --calls` prints the attempts (parse and help tests).

**WP5: the Flight Player** (`feat(flight)`). Window `flight`, 1180x760, minimum 880x560, `window::protect`; opened by the card or the tray item **Open Flight Player** (run picker). `config/flight.ts` (pure, 30 tests): time to step and back (first step at 0, steps in the same second held, a session under a second), segments, zoom, visible rows, action layer, typed-text masking, tabs that have data. Components: header, stage (Video.js or still), timeline (virtualised, zoom, markers, mini-map), caption line, drawer. Tests: 14 component tests for the states (empty record, unreadable record, no such run, LIVE, no steps, missing video, deleted screenshot), keys, the typed-text hiding, a 200-row list drawing under 20 rows, and a 200-segment track drawing under 40 when zoomed.

**WP6: docs.** `ARCHITECTURE.md` (new section, schema list, CLI), `README.md` (feature row, privacy text, layout), use cases 08 (updated) and 12 (new) and the index, `ROADMAP.md` (R6b status; R6c wording unchanged).

## Measured in a real browser (headless Chromium, dev server, fixtures)

- Fixture run (6 steps, 55 s) opened from `?view=flight&run=...&video=webm`. The player upgraded and played; seeking by clicking the timeline took **25 to 60 ms** per jump (click to not seeking, 12 jumps, WebM VP8, served by Vite).
- The still-image fallback, the typed-text masking in caption, arguments and API bodies, the tabs, the drawer, the picker and Escape behaved as the tests say. No page errors.
- **Not measured:** WebView2, H.264, the asset protocol, `Range` requests through it, real file sizes. The Chromium used here has no H.264, so an MP4 could not be played at all in this environment.

## DECIDE defaults taken

- Typed text hidden by default (the handoff's default), in every view, including the overlay on the picture and the bodies.
- `offerReview` default `always`; `recordSessionVideo` default `off`. Neither has a Settings control yet (`settings.json` only, like `archiveKeepDays`).
- No packaged Video.js skin; own controls.
- "Pill menu" entry became the **tray menu** item "Open Flight Player": the pill has no menu, and a tray item works while the pill is click-through.
- The drawer pushes the content aside instead of covering it (the toolbar stays reachable); it still slides in over 220 ms, none under reduced motion.
- The Tools tab lists the `rune_<tool>` call for runs by an outside agent only; kit tools and skills have no tab until they have data.
- Voice clips are not listed in the Files tab (no per-step record of them exists).

## NOT CERTAIN

- **The Windows encoder**: never run. Check: finish a task with `recordSessionVideo` off, wait for the worker (5 s polls, only when idle), see `session.mp4` and the `video` entry in `session.json`; open it in any player. If `video.unavailable` is set, read the reason.
- **MP4 in the Tauri webview**: whether H.264 plays, seeks fast and loads through the asset protocol with an empty static scope plus per-file grants. Check in the player: play, click far along the timeline, watch the time.
- **Size per minute** of a real video, and the **cost of `motion` mode** (CPU, disk, effect on a task's speed; run the eval with it on).
- Step times are one second apart, so the picture and the timeline can disagree by about a second; event markers are placed by the same clock.
- Whether the server returns reasoning text in `reasoning_content` (the Reasoning tab reads it from the stored reply and otherwise says it was not recorded).
- Decision-model, voice and steer calls are not in `calls`.

## Live steps for the maintainer

1. **Card**: run a task. When it ends the card appears above the orb with the outcome, time and steps. Dismiss, Review, and doing nothing for 15 s (dots run out). With `offerReview` set to `failures`, a successful task shows no card.
2. **Player**: Review opens the window. It is absent from OBS and the Snipping Tool (and visible with demo mode on). Play, pause, Left/Right, J/K, D, A, T, Esc. Click and drag the timeline; the picture follows. Zoom with the wheel.
3. **Retry**: make the server fail once (stop it for a moment during a step). The step shows a retry tick and the API tab lists two attempts with the wait between them. `rune-studio-cli run <id> --calls` shows the same.
4. **Outside agent**: let Claude Code run a task; the header shows its name and the Tools tab lists the `rune_*` call.
5. **Old run**: pick a run from before this milestone in the picker; no video, a note says so, the screenshots play, no API tab.
6. **Typed text**: a task that types text. It is masked in the caption, the picture, the arguments and the API bodies until **Typed text: hidden** is clicked.
7. **Missing files**: delete a screenshot and the video of one run; the player says so in one line.
8. **Motion mode**: set `recordSessionVideo` to `motion`, run a task, look at the size of `motion/` and the task's speed against a run with it off.

## Not seen

Everything on a real Windows screen: the card, the window, protection from capture, the tray item, the encoder, H.264 playback and the asset protocol.