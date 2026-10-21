# Handoff: Rune ecosystem architect and orchestrator

Written 2026-10-21 for the person or agent taking over this role from the current orchestrator (an AI assistant working with Shirsendu Bairagi, `@shba007`, the owner). It does not repeat documents that already exist: it says what the role is, how to work with the owner, where everything is, where things stand and what to do next. Read sections 1 to 3 first, then section 6.

## 1. The role

**Orchestrator and verifier, not coder.** You write handoffs for other AI agents (and project managers), verify what they report, keep the plans honest, and tell the owner in plain words where things stand and what is blocking. You do **not** modify application code unless the owner explicitly says so. You do not set the individual goals of rune-infer, rune-kit or rune-tools; you state what Rune Studio needs from them as a consumer. You may read their repositories to **verify a report**, not to design their work.

Long-term goal (the owner's): a **life harness** built on a universal, self-reflecting AI harness. It works from **evidence**, assumes nothing, expects that the user and the model can both be wrong, can change the trajectory mid-session, collaborates instead of taking over, and automates only what is automatable **after asking**. Trajectory chosen earlier: a reflective personal agent (record, verify, reflect, improve).

## 2. The ecosystem (one paragraph each)

- **Rune Studio** (`shba007/rune-studio`): Windows desktop agent, Tauri 2 (Rust) + Vue 3. Sees the screen, acts, records everything, checks its own work. Your primary project. Branch `develop`.
- **rune-infer**: manager of all models and engines (chat, vision, OCR, speech, decisions), local and cloud, with hot swap ("fits in VRAM stays loaded, otherwise swaps"). Has a library and a TUI. All model work belongs here.
- **rune-kit** (`rune` CLI): manager of all non-model tools; MCP gateway over stdio; generic tool rules. Has a library; no TUI yet.
- **rune-tools**: the individual plugins (filesystem, git, time, fetch, memory, browser, and others) and each tool's own rules and UI blueprint.
- Decisions by the owner that bind everything: models are **interchangeable** (for example Gemma 4, MiniCPM-o 4.5, Bonsai 2 27B), so nothing may assume one model; sensitive life data (finance, health, mind) goes only to a **local** model; tools describe their UI as data (**A2UI**-style JSONL blueprints), Studio renders it with its own design system and never generates UI code at runtime; all Rune interfaces, including the TUIs, follow the **Nothing-phone dot design language**.

## 3. How the owner works (follow these)

- "Your job is not to code, but to orchestrate and verify."
- **Too much back and forth is unwelcome.** Batch work; check once at the end of a batch.
- **Studio agents work directly on `develop`. No extra branches.** One commit per work package.
- **Ecosystem documents and test guides were first given to the owner directly (as files), not pushed into the repo.** On 2026-10-21 the owner asked for all of them, including this handoff, to be pushed into `docs/orchestrator/`; that is now where they live. New ones: ask the owner whether to push or hand over directly. Studio's own docs (roadmap, architecture, use cases) live in `docs/` and were pushed to `develop` when asked.
- **Keep answers short and plain.** The owner said twice that the volume of files and jargon was confusing. When the status is complicated, give three or four numbered steps first, details after.
- Mark anything unverified **NOT CERTAIN**. Never claim a live behaviour that nobody observed. Report test failures with the output.
- Use they/them for people whose pronouns you do not know.
- The owner runs the live tests on Windows and spins the agents; you hand over files and they run them.

## 4. Method (the loop that works)

1. Write a handoff: goal, preconditions, what is known and what is NOT CERTAIN, work packages in order, "do not" list, report format. Every handoff tells the agent to check its facts against its own code, add tests, and never claim what it did not run.
2. The owner gives it to an agent; the agent works and writes a report (`docs/handoff/<name>-report.md`).
3. **You verify**: read the report, spot-check the code, **re-run the tests yourself** (reports have been wrong: rune-infer claimed 121 passing and 0 failing, a re-run showed one failure), look for contract drift between projects, and turn gaps into a follow-up handoff.
4. Keep three words separate in every status: **built**, **unit tested**, **seen on a real machine**. Only the last closes a milestone.

## 5. Where everything is

**In the Studio repo (`develop`)**
- `docs/ROADMAP.md` (plan and the current milestone status table), `docs/ARCHITECTURE.md`, `docs/AGENT-GUIDE.md`, `docs/models.md`, `docs/wire-audit.md`, `docs/evals/README.md`.
- `docs/use-cases/README.md` and pages 01 to 12 (one self-contained page per use case, numbered "How to use").
- Reports in `docs/handoff/`: `M1-report.md`, `R6b-report.md`, `C1-report.md`. Earlier reports (R0 to R6a, F1, the 2026-10-19 field test report) were removed and are in git history (for example the field test at `0531bec^:docs/field-test-report-2026-10-19.md`).
- Reports in the other repos, on their `develop` branches: `rune-kit` `docs/handoff/studio-handoff-report.md`, `rune-infer` `docs/handoff/studio-handoff-report.md`, `rune-tools` `docs/handoff/rune-tools-report.md`.

**In `docs/orchestrator/` (pushed 2026-10-21; this folder, flat, filenames as referenced below and in the documents themselves). `README.md` there is the index.**
- Plan and context: `00-ecosystem-plan.md`, `rune-life-harness-milestones.md` (L0 to L10), `rune-design-language.md`, `rune-infer-tui-spec.md`, `milestone-status-2026-10-21.md` (the audit), `field-test-R0-R4.md` (the earlier tester guide).
- Handoffs to the other projects: `rune-infer-handoff.md`, `rune-kit-handoff.md`, `rune-tools-handoff.md` (the v5 versions), and the follow-ups `rune-infer-followup-handoff.md`, `rune-kit-followup-handoff.md`, `rune-tools-followup-handoff.md`.
- Handoffs to Studio agents: `rune-studio-M1-model-profiles.md`, `rune-studio-L0-life-foundations.md`, `rune-studio-K1-kit-client.md`, `rune-studio-C1-closeout.md`, and the shared `rune-studio-contract-addendum.md` (the real field and header names from the three projects).

## 6. Where things stand (2026-10-21)

Authoritative: `milestone-status-2026-10-21.md` and the table in `docs/ROADMAP.md`. In short:

- **Complete:** R1 (flight recorder). R0 except two items (wire table now restored; the elevated-window run still open).
- **Built and tested, not seen on a real machine:** F1 (19 field-test fixes), R5 (agent identity, demo mode, idle shell), R6a (analysis overlay, boxes always on), R6b (session video, review card, Flight Player), M1 (model profiles and roles), C1 (contract and settings close-out, commit `944985b`).
- **Not complete:** R2 (no valid baseline), R3 (verifier not measured; false contradictions on Notepad), R4's box probe never run, L0 (store, consent, encryption and part of the local-only policy landed; redaction, wellbeing basics and docs remain).
- **Not started:** K1 (kit client), K2 (A2UI), R6c, R7 onward, L1 to L10.
- **Verified by re-running tests (2026-10-21):** Studio `cargo test --lib` 388 passed, 1 failed (an L0 test that needs a real OS keyring; environmental, but the test should not depend on a keyring); rune-kit 23 passed; rune-infer 119 passed and 1 failed (a test that needs GPU memory figures); rune-tools not re-run (reports 228 passed). Studio clippy still shows about 30 warnings and the TypeScript check does not run because of a tool version problem.
- **The one finding that matters most:** the only live observation is the field test of 2026-10-19, which ran **before** the F1 fixes. Everything since is unobserved. A deadlock at every task start survived R0 to R4 for this reason. **Rule in force: no new milestone until a second field test on Windows passes.**

## 7. Open problems by project

- **rune-tools / rune-kit mismatch (blocks the first real kit use):** rune-tools emits `rules` and `inputSchema["x-rune-rules"]`; rune-kit reads only `annotations` (or `[tools.<name>]` in `plugin.toml`), so `rune run --read-only` lists nothing from rune-tools. Fix is in the follow-up handoffs (tools: derive `annotations` from rules, TF-1; kit: accept both annotation spellings, KF-1 and KF-7). Kit emits `readOnly` where the MCP spec, to the orchestrator's knowledge, says `readOnlyHint` (not re-checked). No release of any of the three is confirmed.
- **rune-infer:** a running background request is **not** pre-empted by a foreground one (IF-1), so a real idle-reflection job is not safe; model attribution headers only on chat routes (IF-2); the speech route and the model support matrix are half answered (IF-3, IF-4); idle-unload configurability and the 783 MiB VRAM question unanswered (IF-5); "calibrated" decisions unproven (IF-6); TUI needs a visual review from screenshots (IF-7).
- **Studio:** L0's remaining packages and the keyring-test fix; clippy and the TypeScript check; box accuracy unmeasured and boxes cannot be turned off by decision; the Windows H.264 encoder has never run (the biggest R6b risk); brand colours and marks for Antigravity, ChatGPT and Codex are missing.
- **Human-only checks never done:** an elevated Notepad, the owner's own voice, 125 and 150 percent scaling, a second monitor, lock screen, UAC and sleep, the 45-run baseline, the three-mode verifier suite, the box probe, `session.mp4` creation and playback in WebView2, `motion` mode cost.

## 8. What to do next, in order

1. **Gate G1 (field test round 2) is the priority.** Offer the owner a one-page checklist (not yet written; the earlier guide covered only R0 to R4). Priority: a task starts and the confirm card works (approve a delete in Explorer, it must reach the file), the stuck detector and `open_app` ("open Notepad and write hello" in about 60 seconds), R5 on screen, R6a on screen, an MP4 produced and played, two models with a swap, then the baseline, the verifier suite and the box probe.
2. In parallel, agents only (no screen needed): finish **L0** (reminder: replace the real keyring with an in-memory store in tests; check `life/policy.rs`; run `cargo fmt`); the three follow-up handoffs for rune-infer, rune-kit and rune-tools.
3. After G1 passes: take the baseline per model (R2) and the verifier measurement (R3); then **K1** against a real kit with declared tools; then L0's completion review; then L1 (the strategy session) and the rest of the life track; R6c and R7 follow the original order.
4. Verify every incoming report (section 4) before building on it.

## 9. Decisions waiting for the owner

- Brand colours and marks for Antigravity, ChatGPT and Codex.
- The first model for the baseline.
- The life-track defaults I assumed (all in `rune-life-harness-milestones.md` section 8): life module inside Studio; a database as source of truth with Obsidian as a generated view and an input; domains first (goals, skills, fitness, productivity; finance read-only from imported files; mental health light-touch, no diagnosis); passive activity sampling off; local-only enforced and not overridable; Solo Leveling as a language skin only; for the owner alone until L9.
- Whether A2UI/K2 should be added to `docs/ROADMAP.md` (it is only in the ecosystem plan so far).
- The wellbeing text and hotline information are **placeholders marked OWNER TO REVIEW** in L0; the owner must supply them.

## 10. Working environment notes (the sandbox)

- Studio is checked out at `/home/user/rune-studio`. The other repos are read-only clones under `/home/user/shba007/` (rune-infer, rune-kit, rune-tools); the local branches there are stale, so fetch with `git fetch --depth=50 origin develop` and read `FETCH_HEAD`. Use `add_repo` only to confirm read access.
- Rust: use `cargo +1.98.1` (the default toolchain is 1.97). The Linux webkit libraries are installed. The sandbox has **no Windows, no GPU, no OS keyring and no model server**, which explains the environmental test failures.
- Long builds: run with `nohup ... &` and a Monitor on the log; `sleep` chains are blocked. For rune-kit tests set `CARGO_HUSKY_DONT_INSTALL_HOOKS=true`.
- Git pushes of new refs work; deletes are refused (403) through the proxy. A merged PR cannot be reused; branch deletions earlier closed a PR by mistake once (never put "safe now" and "after the PR merges" commands in one block).
- Files for the owner go through the file-send tool from the scratchpad directory. The ecosystem and orchestrator documents were later pushed to `docs/orchestrator/` at the owner's request; a copy there may be newer than a delivered file, so treat the repo copy as the source from now on.

## 11. Mistakes already made (do not repeat)

- Giving the owner a command list that mixed safe and unsafe steps; they ran all of it.
- Pushing a test guide into the repo after being told to hand it over directly.
- Writing an agent handoff that told it to create a branch when the owner wanted `develop`.
- Letting an agent build against a guessed contract: M1 guessed rune-infer's header names, rune-tools built a rules format kit does not read. **Before handing a milestone that depends on another project, confirm the real contract first and give the agent the addendum.**
- Trusting a green report. Always re-run.
- Sending many files at once. Prefer one short summary with "give these files to these agents".
