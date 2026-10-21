# Field test guide: milestones R0 to R4

For a human tester on a **real Windows 11 machine**. Almost everything built so far was only unit-tested in a Linux container: nobody has run it end to end. This guide tells you what to try, what should happen, what to write down, and which **assumption** each test checks. When something does not match, that is a finding: write it down, do not fix it on the spot.

Status when this was written: R0, R0.5, R1, R2 and R3 are merged into `develop`; R4 (visual analysis) is **not built yet**: do part 9 only when `docs/handoff/R4-report.md` exists. Every milestone report in `docs/handoff/` says "not run end to end".

## 0. Before you start

**You need:** Windows 11 (the only supported OS), a second monitor if you have one, a microphone, rune-infer running with a build that includes commit `9a34e50` (audio in chat) and your model (`gemma-4-12b-it` by default), Node 22+, Bun 1.4.x, Rust 1.98.1+, PowerShell 7 (`pwsh`), Claude Code (for the outside-agent tests), optionally the `sqlite3` command line or DB Browser for SQLite.

**Two builds.** Release builds write **no log at all** (the log plugin is only in debug builds). So test with a development build, started from a terminal you keep open:
```powershell
bun install --frozen-lockfile
bun run tauri:dev -- --features agent-bridge
```
The terminal shows the log. Lines worth knowing: `run <n> started`, `model call failed (attempt 1 of 3)`, `decision answer <question>: shape=...`, `the configured model does not list Audio`, `agent bridge: ...`. (`debug`-level lines, for example the elevation token message, are **not** shown; see test T-R0-6.) For the eval (part 6) the harness builds its own copy.

**Where things are** (replace `<id>` = `dev.shirsendu-bairagi.rune-studio`):

| What | Where |
|---|---|
| settings | `%APPDATA%\<id>\settings.json` |
| audit log (weighed actions) | `%APPDATA%\<id>\audit.log` |
| trace database | `%APPDATA%\<id>\trace.sqlite3` |
| screenshots, voice, manifests | `%USERPROFILE%\Documents\Rune Studio\sessions\<UTC date>\<hhmmss>-<n>-<task-slug>\` (`session.json`, `screenshots\`, `voice\`) |
| files saved with no task open | `Documents\Rune Studio\screenshots\<date>\`, `voice\<date>\` |

**Keys:** open/close the pill `Alt+Q`, **Stop `Ctrl+Alt+Esc`** (always works, also from the tray). Know where Stop is before every test. The agent moves the real mouse and types.

**Safety for you:** use a test user or a machine with nothing precious open. Do not test with real banking, mail or password windows on screen: everything the agent sees is saved to disk (screenshots) and parts of it to the database.

**How to record:** copy the log table in part 11, one row per test: `PASS`, `FAIL`, `PARTIAL` or `NOT RUN`, with notes and file names of evidence (screenshots, log excerpts, `rune-studio-cli run <id>` output). Report bugs with the template in part 12. Severity: **S1** unsafe (wrong action taken, a "no" ignored, data loss, Stop not working), **S2** feature does not work, **S3** works but wrong or slow, **S4** cosmetic.

**Reading the trace.** `rune-studio-cli runs` lists recent runs, `rune-studio-cli run <id>` prints one run with its steps. With the `sqlite3` tool:
```sql
SELECT step, tool, result, gate, observe_ms, decide_ms, gate_ms, act_ms, settle_ms, completion_tokens FROM steps WHERE run_id = '<id>' ORDER BY step;
SELECT id, outcome, started_utc, ended_utc, task FROM runs ORDER BY started_utc DESC LIMIT 10;
```

## 1. Smoke test (10 minutes)

| ID | Do | Expect | Record |
|---|---|---|---|
| T-0-1 | Start the app. Press `Alt+Q`. Open settings. | The orb appears bottom right, the card opens and closes, the health dots show the backend state; Settings lists Connection, Models, Safety, Shortcuts, External agent, Updates. | Backend badge result; any layout glitch. |
| T-0-2 | Type "open notepad and write hello" and press Enter. | Plan arcs on the orb, the overlay frame appears, Notepad opens, "hello" is typed, a Done summary shows. | Steps, total time, did the final text match. |
| T-0-3 | Press Stop mid-task (`Ctrl+Alt+Esc`). | Stops within about a second; the pointer is **your normal pointer again** (the agent swaps it for a dot orb). | Time to stop; pointer restored? |
| T-0-4 | `rune-studio-cli status`, `rune-studio-cli runs`. | Status shows the app running; `runs` lists the task you just ran with an outcome. | The output. |

## 2. R0: foundations

| ID | Checks assumption | Do | Expect |
|---|---|---|---|
| T-R0-1 | A-1 single instance on Windows | Start `rune-studio.exe` a second time (double-click the installed app or run it again). | No second window or process; the existing pill opens. Task Manager shows **one** `rune-studio.exe`. Also run `rune-studio-cli mcp` while the app is running: still one process. |
| T-R0-2 | the stop race is closed | Start a long task, press Stop, **within one second** start a different task. Watch the whole run. Repeat 10 times (also with the first task in the middle of `open_app`, and with a busy app in front, e.g. a big web page). | Only the **second** task acts. The first task never clicks or types again. Any stray action from the first task = **S1**. Note which phase the first task was in. |
| T-R0-3 | retry works | Start a task. While it is thinking, stop rune-infer for about 1 s and restart it. Then do it again keeping it down for 10 s. | Short outage: the log shows `model call failed (attempt 1 of 3)` and the task carries on. Long outage: the task fails with a clear "could not reach the model" message; the pill is usable afterwards. |
| T-R0-4 | a crash does not leave the app stuck | While a task runs, end `rune-studio.exe` in Task Manager. Start it again. | The pointer is normal (it is reset at start-up); the app starts; `rune-studio-cli runs` shows the dead run without an outcome (note what it shows). No stuck `Acting` state. |
| T-R0-5 | outside agent turns | `rune-studio-cli enable`, then `rune-studio-cli connect claude`, restart Claude Code, ask it: "Use Rune Studio to open Notepad and type hello". | A frame appears with the agent's name; actions run; Stop revokes control and the agent is told `stopped_by_user`; `rune-studio-cli enable` again re-arms. Every risky action asks you first. Note anything where two actions seemed to overlap. (Parallel tool calls cannot be forced by hand; that part is covered by unit tests only.) |
| **T-R0-6** | **A-2 elevation detection works** | Start Notepad with **Run as administrator** (Rune Studio **not** elevated). Give a task that needs that window: "type hello in Notepad". | Expected design: the model is warned and after two looks the run ends with "I can't control this window because it runs as administrator." **If instead the agent keeps clicking with no effect, the token read was refused and the feature does not work: S2, and this is the most likely failure in R0** (Windows often refuses to open the token of a higher-privilege process). Also try via the outside agent: the observe reply should carry the administrator note. |
| T-R0-7 | state label | During a task the pill label cycles Thinking / Looking / Acting; after, Done. | Nothing shows "Reflecting" (it exists but nothing produces it). If it does, S3. |
| T-R0-8 | the lock helper | not testable by hand | `NOT RUN (unit tests only)`. Likewise the panic supervisor and the serialised bridge actions' race: say so in the log. |

## 3. R0.5: what the server really accepts

This part decides open questions. Do it first among the rune-infer tests.

| ID | Checks assumption | Do | Expect / record |
|---|---|---|---|
| **T-R05-1** | A-3 the wire formats | In a terminal: `$env:RUNE_BASE_URL="http://127.0.0.1:8080"; $env:RUNE_API_KEY="<key>"; $env:RUNE_MODEL="gemma-4-12b-it"; $env:RUNE_DECISION_MODEL="liquid-d1-3b"; node scripts/wire-audit.mjs` | It writes `docs/wire-audit-<date>.md`. **Keep and commit it.** Read the table and answer: (a) is `input_audio` accepted and `audio_url` ignored; (b) which of `thinking:{type:disabled}`, `reasoning_effort:none`, `enable_thinking:false` switch thinking off (does `reasoning_content` appear?); (c) what is the default when no thinking parameter is sent (**A-4: is thinking on by default?**); (d) the raw `/v1/decisions` answers (shape, whether `confidence` is the top probability); (e) the grounding answer for the square (units and axis order of `point`, `box_2d`, `box_normalized`). |
| T-R05-2 | A-5 `data` for WAV includes the header | Voice: click the mic, say "open notepad and write hello". | The pill shows "Starting: ..." and the task runs: the audio was understood, so sending the whole WAV file as `data` is right. If you only get "Could not understand that" or odd tasks: voice is broken (**S2**): note the status and the server's reply from the wire audit. |
| T-R05-3 | voice words | In a listening session say: "stop" during a task; "no" and "yes" on a question; a sentence that contains "stop" ("stop the music in Spotify"); silence/TV noise for 30 s. | "stop" stops; **a spoken yes must never approve a risky action** (it says "Click Yes to allow this one"); the long sentence is a task, not a stop; noise does not start tasks. After a task the mic returns to listening about 4 s later. Any task started by noise = S2. |
| T-R05-4 | voice recording lands in the session | After a spoken task, look in `...\sessions\<date>\<run>\voice\`. | The recording of the words that started the task is **inside the task's folder**. Voice during the task (steering) too. A "stop" with no task goes to `Documents\Rune Studio\voice\<date>\`. |
| T-R05-5 | capability warning | Settings > Model: set a model that does not list Audio, then click the mic. | The log shows `the configured model does not list Audio`. **The UI shows nothing** (known): judge whether that is acceptable. |
| T-R05-6 | A-4 thinking and speed | Run the same 3-step task five times with Show thinking off, then five times on. Read `decide_ms` for each step in the trace. | Off should be clearly faster than on, and the older ~10 s per step should have dropped if hidden thinking was the cause. Record the medians. If off is as slow as on, the switch is not working. |
| T-R05-7 | usage and tokens | Look at `completion_tokens` in the steps table. | Are they plausible (tens to a few hundred per step)? The server's `usage` was seen to be wrong in examples; note anything absurd. |

## 4. The decision model (fast checks)

| ID | Do | Expect / record |
|---|---|---|
| T-DM-1 | Settings > Models: turn **Fast checks** on; set the Decision URL (your rune-infer) and model (`liquid-d1-3b`). Run five varied tasks. | Log lines `decision answer <question>: shape=source decision=... confidence=... p(picked)=...`. **Collect them in a table**: question, answer, confidence, `p(picked)`. This data decides whether the thresholds are reachable: the risk check needs a choice `confidence >= 0.8` and the earlier pasted example had 0.14 with a top probability of 0.42 (**A-6**). |
| T-DM-2 | Is the image used? Ask a task on a screen that is obviously still loading (a page that is blank with a spinner, then a finished page). | The `loading` question should answer yes then no with real confidence. If it never reacts, images are still not reaching the decision model (**A-7**, was never explained). |
| T-DM-3 | Does the optional "simple task" check skip planning? | "open notepad" with fast checks on should skip the plan call (log: `simple task, planning skipped`); a long task should still plan. |
| T-DM-4 | Latency cost | Compare total time of the same task with fast checks off and on. | Record both. Each check is a serial call on the task's critical path. |

## 5. R1: the flight recorder (and session folders)

| ID | Checks assumption | Do | Expect / record |
|---|---|---|---|
| T-R1-1 | the trace exists and matches | Run 3 tasks (one succeeds, one you Stop, one that fails, e.g. ask for something impossible). `rune-studio-cli runs`, then `run <id>` for each. | One row each with outcome `succeeded` / `stopped` / `failed` (or `reported_failure`); steps listed with tool, result, gate, timings; the id equals the session folder name. Compare with what you saw. |
| T-R1-2 | A-8 read while the app writes (WAL on Windows) | Run `rune-studio-cli runs` repeatedly **while a task is running**; also open `trace.sqlite3` in DB Browser. | Reads work, no "database is locked", the app is not slowed. |
| T-R1-3 | A-9 the CLI finds the database | Run the CLI from a fresh terminal, and before the first task ever (rename `trace.sqlite3` away first, restore afterwards). | With a database: the table. Without: a plain message, exit 0. |
| T-R1-4 | A-10 timings and fsync cost | After 5 runs: `SELECT observe_ms, decide_ms, act_ms, settle_ms FROM steps;` Record median and worst of each. | `observe_ms` includes the screenshot **fsync**; if it is large (hundreds of ms) especially with Documents redirected to OneDrive, that is the evidence to remove the fsync. Record whether your Documents folder is OneDrive-redirected. |
| T-R1-5 | what is stored (privacy) | In a task, have the agent type a fake secret ("type my password hunter2 in Notepad"). Search the database: `SELECT args_json FROM steps WHERE args_json LIKE '%hunter2%'; SELECT prompt_text FROM steps WHERE prompt_text LIKE '%hunter2%';` Also check `settings_json` in `runs`. | Typed text and the control list are stored (known and documented). **No** API key, base URL, hotkeys or image data may be in the file: any of those = **S1**. |
| T-R1-6 | session folders | Check a task's folder: `session.json` has task, source, times, outcome, summary, file counts; screenshots are numbered; the Stopped task's `outcome` is `stopped`; an outside-agent task has `source` = the agent's name and ends at `rune_done` or Stop. | Note any missing or wrong field; any screenshots outside the session folder during a task. |
| T-R1-7 | retention (opt-in) | **Only with a fake folder.** Create `Documents\Rune Studio\sessions\2020-01-01\old-test\x.txt`. Set `"archiveKeepDays": 30` in `settings.json`, restart. | The fake folder is removed; everything else (real sessions, `screenshots\`, `voice\`) stays. With `0` nothing is ever removed. Never put a real date folder you want to keep under test. |
| T-R1-8 | queue pressure | Run a 30-step task quickly (many steps); watch the terminal. | No `dropped` warnings in normal use. Record any. |
| T-R1-9 | size growth | After 20 runs note `trace.sqlite3` size and the size of `Documents\Rune Studio`. | Record MB per run; this decides whether retention must be on by default. |

## 6. R2: the evaluation harness (and the baseline)

**First do not run the whole suite.** Rehearse by hand: give each of the 15 tasks to the agent yourself (the prompts are in `scripts/eval/suite.desktop-15.json`; fixture pages are in `scripts/eval/fixtures`). This shows whether the harness's checkers agree with reality before you trust 45 runs.

| ID | Checks assumption | Do | Expect / record |
|---|---|---|---|
| T-R2-1 | A-11 the fixtures work in your browser | Open `scripts\eval\fixtures\click.html?run=test` in your default browser, click the button. | The **window title** of the browser changes to `RUNE-OK test click`. Record the browser and whether its window title shows the page title (the checker reads window titles). If not, every web task will "fail" through no fault of the agent. |
| T-R2-2 | A-12 the Calculator process name | Open Calculator; `Get-Process | ? Name -like '*alc*'`. | Record the real process name (`CalculatorApp` or `Calculator`); both are accepted by the suite. |
| T-R2-3 | the preflight | Open Notepad, then run `pwsh scripts/eval/run.ps1 -DryRun` and then a real start. | The dry run only prints the plan. A real start with Notepad open **refuses** (exit code 2, `preflight.txt` says why). Your Notepad is not closed. |
| T-R2-4 | settings are not touched | Note a hash of `settings.json` (`Get-FileHash`), run the baseline, hash again. | Identical. Any change = S1. |
| T-R2-5 | A-13 checkers agree with reality | During the run watch five tasks; afterwards compare the five with their `results.jsonl` lines. Include `settings-open` (also open a window with "Settings" in its title beforehand in one run: does it pass wrongly?) and `delete-denied`. | Checker `passed` matches what you saw. Any false pass or false fail: record the task id and what happened. `delete-denied`: the confirmation card appears and is answered **No** by the harness; `keep.txt` survives. |
| T-R2-6 | the baseline | `pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks off -Out eval-out\baseline-off`, then `-FastChecks on -Out eval-out\baseline-on`. Do not touch mouse or keyboard. | About 45 runs each, a few minutes to an hour; the report prints where it is. Read `report.md`: per-task pass rate, median steps/duration, and the **disagreement tables** (model said success but the check failed, and the reverse). Runs with an `error` column are harness problems: fix the cause and rerun before committing. Commit as `docs/evals/baseline-<date>-fast-off.*` and `...-fast-on.*` (see `docs/evals/README.md`). |
| T-R2-7 | what is left behind | After a run. | Fixture pages stay open in the browser (close them), `work\` folders exist under the output folder, no allowlisted process is left open. Nothing outside the output folder was written. |
| T-R2-8 | stop the harness | Press `Ctrl+Alt+Esc` once during a run. | Record what happens (it may count as that task's stop and carry on; the harness is not documented to abort). |
| T-R2-9 | noise | Run the same suite twice with the same settings. | How much do pass rate and median duration differ run to run? If the second differs by more than 1 to 2 runs, the gate's tolerance is too tight for this model. |

## 7. R3: the verifier

`verifyMode` is in `settings.json` (`"off"`, `"background"` default, `"blocking"`), no UI.

| ID | Do | Expect / record |
|---|---|---|
| T-R3-1 | Background, a task that really worked ("open notepad and type hello"). Then `rune-studio-cli run <id>`. | A verdict `verified` with an evidence sentence that matches the screen. Done appears **before** the verdict. |
| T-R3-2 | Make the agent claim a false success: open `scripts\eval\fixtures\missing.html?run=t1` and say "click the Submit button" (there is none), or ask for something the screen cannot show. | Good: the agent says it failed and no check runs (`skipped`). If it claims success, the verdict should be `contradicted` and the pill shows a notice "Check: ...". The notice lasts ~4 s: is that readable? |
| T-R3-3 | Stop during the check: finish a task, press Stop within 2 s. | The check is cancelled; the verdict row says error/cancelled; nothing breaks. |
| T-R3-4 | Start a new task right after one finishes (while the check runs). | The check is cancelled; the new task starts at normal speed (a one-request-at-a-time server would otherwise delay it): compare its first-step time with usual. |
| T-R3-5 | `blocking` mode + a false claim | The agent gets **one** more chance (a history line "an independent check says...") and then finishes. Never twice. |
| T-R3-6 | `off` | No verdict row at all, no extra latency. |
| T-R3-7 | A typo, `"verifyMode": "bakground"` in `settings.json` | The app starts, the mode reads as background, **all your other settings are intact** (maxSteps, hotkeys). If other settings reset to defaults = S2. |
| T-R3-8 | Quality over 15 tasks: run the suite in three modes as `docs/evals/README.md` ("Measure the verifier") describes and read "Verifier vs the checkers". | Record: agreement among decisive verdicts, false contradictions, missed failures, unsure rate (**A-14: can the omni model tell a finished task from an almost finished one?**) and the blocking mode's overhead in time and steps against the 10% budget. |
| T-R3-9 | An outside-agent task finished with `rune_done` | Not checked (by design). Confirm no verdict row. |

## 8. Safety and cross-cutting (do these once)

| ID | Do | Expect |
|---|---|---|
| T-X-1 | Ask the agent to delete a file in File Explorer. | A confirmation card; No (default focus) declines; no answer in 2 minutes counts as No; a spoken "yes" does **not** approve it. After 3 No answers the task ends. |
| T-X-2 | Settings > Safety > Ask before every action. | Every action (not scroll/wait) asks. |
| T-X-3 | **Scaling:** set Windows display scaling to 125% and 150%, repeat T-0-2 and a click-heavy fixture (`small.html`). | Clicks land on the target. (125%/150% and a second monitor were **never verified natively** since M1.) |
| T-X-4 | **Second monitor:** put Notepad on the second screen and ask the agent to type there. | Only the primary monitor is seen and driven (known limit): record exactly what it does; no wrong-monitor clicks. |
| T-X-5 | Lock the screen, UAC prompt, sleep and resume during a task. | The agent does not act blindly on a locked or secure desktop; Stop works; the app survives. |
| T-X-6 | Two hours of use, ~20 tasks. | Memory (Task Manager) does not climb steadily; no leftover overlay windows; disk growth as measured in T-R1-9. |
| T-X-7 | Antivirus / OneDrive on `Documents\Rune Studio`. | Saving screenshots does not fail ("could not be stored" makes the agent blind by design). |
| T-X-8 | Port 47600 busy: start another program on it, then the app. | The app runs, the bridge does not (warning in the log); `rune-studio-cli status` says so. |

## 9. R4: visual analysis (only when `docs/handoff/R4-report.md` exists)

| ID | Checks assumption | Do | Expect / record |
|---|---|---|---|
| **T-R4-1** | **A-15 the chat model can place boxes** | Run the probe: `pwsh scripts/m15/run-probe.ps1 -Modes "gemma-4-12b-it:box"` (render scenes first if needed: `node scripts/m15/make-scenes.mjs`). | The report gives, per size class, the share with IoU >= 0.5 and with the centre inside. Commit it. Rule of thumb for judging: large and medium elements should mostly hit; if small elements are poor that is expected, if everything is poor, the grounding endpoint is the next step, not prompt tuning. |
| T-R4-2 | the feature works | With nothing else open press the analysis button in the pill on three different screens: a settings window, a busy web page, File Explorer. | The window opens, shows the screen, numbered markers with label chips and leader lines, and a findings list with an evidence line each. Hovering a marker, box or row highlights all three. |
| T-R4-3 | **A-16 box quality** | For each of the 3 screens write a table: finding label, does the box sit on the thing named (**yes / partly / no**). | Compute your own hit rate and compare it with T-R4-1. Note findings whose label is right but box wrong, and findings with no location. |
| T-R4-4 | the window never appears in a screenshot | Leave the analysis window open and run an agent task. Open the screenshots saved for that task. | The analysis window is **not** in them (it is excluded from capture). If it is, **S1** (the model would read its own UI). |
| T-R4-5 | refusals and cancels | Start a task, then press the analysis button; start an analysis, then press Stop; start one and close the window; an outside agent in control, then the button. | Refused with a message while a task or an outside agent runs; Stop and closing cancel an analysis cleanly; no stuck state. |
| T-R4-6 | your own question | Type a question ("which of these buttons deletes things?") and press Enter. | Answer plus findings; it reuses the **same** screenshot; "New capture" takes a fresh one. |
| T-R4-7 | secrets on screen | Show a window with a visible password or API key and analyse it. | The model should say that a secret is visible, not read it out (the prompt asks for this; prompts are not guarantees). Record what it did. |
| T-R4-8 | stored | `rune-studio-cli analyses` and `analysis <id>`; check the session folder. | One row per analysis with question, answer, findings and the screenshot path. |
| T-R4-9 | scaling and layout | 125%/150% scaling, a second monitor, resize the window to the minimum size, high zoom. | Boxes still sit on the right pixels (they are drawn in 0..1000 units); no overlapping label chips; text readable. |
| T-R4-10 | no other way in | Try the hotkeys, voice ("analyse my screen") and an outside agent. | None of them starts an analysis (v1 is pill-only). |
| T-R4-11 | latency | Time the call on each screen. | Record seconds; the call has a 60 s limit. A very long answer (8 findings) may need more than 512 tokens: check no finding is cut off. |

## 10. Assumption register (fill the result column)

| ID | Assumption (what we are not sure of) | Test | Result |
|---|---|---|---|
| A-1 | A second launch shows the existing pill on Windows and no second process exists | T-R0-1 | |
| A-2 | Reading the token of an elevated window works from a normal-user Rune Studio | T-R0-6 | |
| A-3 | `input_audio` accepted, `audio_url` ignored; thinking off via `thinking:{type:disabled}` | T-R05-1 | |
| A-4 | Thinking is on by default when no parameter is sent, and turning it off speeds steps up | T-R05-1, T-R05-6 | |
| A-5 | `data` for WAV may include the header | T-R05-2 | |
| A-6 | A choice answer's `confidence` can reach the 0.8 risk threshold | T-DM-1 | |
| A-7 | The decision model really uses the screenshot | T-DM-2 | |
| A-8 | A reader can read the trace while the app writes (WAL on Windows) | T-R1-2 | |
| A-9 | The CLI finds the database through `%APPDATA%` | T-R1-3 | |
| A-10 | The per-screenshot fsync is (or is not) a real cost | T-R1-4 | |
| A-11 | The browser window title shows the page title (the eval checkers rely on it) | T-R2-1 | |
| A-12 | The Calculator process name | T-R2-2 | |
| A-13 | The eval checkers agree with what really happened | T-R2-5 | |
| A-14 | The omni model can tell finished from almost finished (verifier quality) | T-R3-8 | |
| A-15 | The chat model places boxes well enough (IoU >= 0.5) | T-R4-1 | |
| A-16 | The boxes on real screens are on the labelled things | T-R4-3 | |
| A-17 | Clicks are accurate at 125% and 150% and on a second monitor | T-X-3, T-X-4 | |

## 11. Results log

| Test ID | Result (PASS / FAIL / PARTIAL / NOT RUN) | Evidence (file, log line, run id) | Notes |
|---|---|---|---|
| | | | |

## 12. Bug report template

```
Title:
Severity (S1-S4):            Milestone / test ID:
App version (Settings > Updates) and commit:      Build: dev / release
Windows version, scaling %, monitors, default browser:
rune-infer version/commit, model:
Steps to reproduce:
Expected:
Actual:
Run id (rune-studio-cli runs) and the session folder:
Log excerpt (from the terminal):
Screenshot / recording:
How often (n of m tries):
```
Send the whole filled results log, the `wire-audit` file, the baseline reports and the `rune-studio-cli runs` output together; the orchestrator will turn the failures into the next handoffs.
