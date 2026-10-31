# Handoff to the rune-tools project manager

## Agent brief (read first)

You are an AI coding agent working **inside the rune-tools repository**. The owner gave you this document. It was written by the Rune Studio orchestrator **from outside your repository**: what it says about rune-tools comes from Studio's observation (a field test and earlier reading) and **may be wrong or out of date**. Your first job is to check every statement about rune-tools against your code and say where it is wrong.

How to work:

1. Read this whole document and your repository's own contributing rules, branch rules and test commands. **Follow your repository's rules**, not any in this document. Do not push to a protected or release branch, and do not cut a release.
2. For every request below, find the current state in your code (exists, partly, missing) and write it down before you change anything.
3. **Build** the items in the build list (TOOL-1 (first for filesystem, git and time) and TOOL-6; TOOL-2, TOOL-4, TOOL-5 and TOOL-10 as written statements; the rest are Questions or Process). For the others, report feasibility, and implement one only if it is small and clearly safe; otherwise leave it for the owner. Items marked Question or Process are answered in your report, not built.
4. Where a request offers options, choose the simplest that works, state the choice and why, and keep it compatible with existing clients (add, do not break; announce anything that changes behaviour).
5. Add tests for what you build. Run your repository's tests, lints and formatters and report the numbers before and after. **Never claim a behaviour you did not run.**
6. Mark anything you could not verify **NOT CERTAIN**. Do not invent facts about models, protocols or other projects.
7. Do not change the goals or scope of rune-tools beyond this document. If a request conflicts with your code or design, say so and propose the smallest alternative.
8. Finish with a report file in your repository's usual place for such notes (for example `docs/handoff/<name>-report.md`) containing: the corrections to this document, the state of each request, what you built with test evidence, your answers to the Questions, and the filled response table at the end of this document.

## 0. About this document

Written by the Rune Studio orchestrator for the project manager of this project. It does **not** set your goals, scope or roadmap, and it does not describe your code: those are yours. It states what **Rune Studio needs from this project as a consumer**, why (with evidence from Studio's field test of 2026-10-19), how Studio will check it, and what Studio does in the meantime. Anything about your project that is written here comes from Studio-side observation and may be out of date or wrong: correct it. Items marked **NOT CERTAIN** are unverified.

You answer each request with one of: **Accept** (with an owner and a target release), **Counter** (what you will do instead), **Decline** (with the reason), or **Question** (what you need to know). Use the table at the end.

## 1. The ecosystem in one page

| Project                   | Role                                                                                                                                                                                                                                                | Who uses it                                                                                          |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **rune-infer**            | The manager of all **models and engines**, local and cloud. Everything that needs a model is done here: chat, audio and speech, vision, OCR, grounding, decisions, embeddings. If the model fits in VRAM it stays loaded, otherwise models swap     | Rune Studio, other clients. Has a library and a TUI (to be redesigned, see `rune-infer-tui-spec.md`) |
| **rune-kit** (`rune` CLI) | The manager of all **non-model tools**, the way rune-infer is the manager of models: installs and runs plugins and skills, runs them as one MCP gateway over stdio, and owns the **generic rules** that apply to any tool                           | Rune Studio (planned MCP client, milestone K1), other agents. Has a library; no TUI yet              |
| **rune-tools**            | The individual tools (plugins): filesystem, fetch, memory, git, browser, email, audio, video, image, slides, figma, time, sequential-thinking, and others. Each tool owns its **tool-specific rules** and its own **UI blueprint** (see A2UI below) | Through rune-kit                                                                                     |
| **Rune Studio**           | The harness: a Windows desktop agent that sees the screen, acts, records everything, checks its own work and (planned) reflects on it, and renders tool UI                                                                                          | End users                                                                                            |

**Ownership rule (decided by the owner):**

- Anything that needs a **model** (OCR, vision, speech, grounding, decisions) is rune-infer's. rune-kit and rune-tools do not host models.
- Anything that is **not a model** (files, web, git, mail, browser, time, memory, UI blueprints) is rune-kit's to manage and rune-tools' to implement.
- **Rules:** a rule that is specific to one tool lives with that tool in rune-tools. A rule that applies to tools in general (defaults, the vocabulary tools declare in, what happens when a tool declares nothing, sandbox and permission enforcement) lives in rune-kit. Rune Studio applies its own safety gate on top of both and never relies on a tool being safe because it says so.

**A2UI (Agent-to-User Interface), decided by the owner.** Tools describe their own user interface as data. Each tool holds its own **JSONL blueprint** for UI widgets; Rune Studio renders those widgets with **its own design system**. The UI code is **not written at runtime** (generating Vue code with a model would add inference time and risk). A2UI is an open protocol (Apache 2.0, created by Google; as found on 2026-10-20 its site lists v0.8 as the stable release and v0.9 as current, a JSON/JSONL declarative format, and a catalog of trusted client-side components; renderers exist for Lit, Flutter, React and others; a Vue renderer was not found, **NOT CERTAIN**). Studio would write its own renderer for its own component catalog. See section 4 and the per-project requests.

**Interfaces (decided by the owner).** Both rune-infer and rune-kit have a **library**. rune-infer has a **TUI** today; **rune-kit has no TUI yet**. The rune-infer TUI is to become cleaner (see `rune-infer-tui-spec.md`) and a future rune-kit TUI is to be built in the same style. Both follow the design principle of Rune Studio, a Nothing-phone style dot interface: dots as the visual language, monochrome with one accent, red only for attention and failure, compact and calm. The shared description, with a suggested terminal translation, is in `rune-design-language.md`. It does not set what the interfaces do, only how they look and feel.

Goal of the whole ecosystem (set by the owner, updated 2026-10-21): a **life harness**, built on a universal, self-reflecting AI harness. Rune does not assume: it works from **evidence** (what you say is stored as what you said, and checked against what tools can observe), it expects that a user can be mistaken and that it can be too, it can **change the trajectory mid-session** when the evidence says the current plan will not work, and it **collaborates** instead of taking over: it improves the user and itself, and automates what is automatable only after asking. Studio's life track (L0 to L10) is in `rune-life-harness-milestones.md` (sent separately).

**Interchangeable models (decided by the owner, 2026-10-21).** Models will be swapped freely, for example **MiniCPM-o 4.5**, **Bonsai 2 27B** and others, and rune-infer supports hot swap. So **nothing in Studio, kit or tools may assume one model.** Studio will keep per-model profiles and per-role model choices (milestone M1) and will ask rune-infer what a model can do instead of guessing. Facts found on 2026-10-21 (secondary and vendor sources, **not verified by running anything**): _MiniCPM-o 4.5_ is an open 9B omni model from OpenBMB (vision, speech in and out, full-duplex live streaming; built from SigLip2, Whisper-medium, CosyVoice2 and Qwen3-8B; Apache 2.0; a GGUF for llama.cpp exists). _Bonsai 2 27B_ is a ternary-weight 27B model from PrismML (reported as based on Qwen3.8-27B, a file of about 5.95 GB, a 262,144-token context, Apache 2.0, reported to need PrismML's fork or its MLX loader rather than stock llama.cpp or Ollama). Sources: huggingface.co/openbmb/MiniCPM-o-4_5, huggingface.co/OpenBMB/MiniCPM-o-4_5-gguf, modemguides.com/blogs/modemguides-blog/run-bonsai-2-27b-locally, cloudprice.net/models/prism-ml-bonsai-2-27b.

Studio's own plan, in order (dates are not committed): R5 agent identity, demo mode, idle reflection shell (done). R6a screen analysis as an overlay (done). R6b session video and Flight Player (done). **M1 model profiles and roles** (next). R6c post-mortems and notes. R7 notes in the prompt, proved by an A/B. R8 trust (redaction, retention). R9 universal abstractions. **K1** the rune-kit MCP client, with every kit tool call going through Studio's safety gate (a risky call waits for the user's Yes; an unknown tool defaults to ask). **K2** A2UI surfaces. **L0 to L10** the life track, starting with L0 foundations (consent, store, trust slice).

## 2. How the projects fit together (the contract)

1. Studio talks to rune-infer over HTTP (OpenAI-compatible, bearer token). Studio talks to rune-kit by spawning `rune run --all` and speaking MCP over stdio (planned). Studio never talks to rune-tools directly.
2. **Safety is Studio's job at the point of action, but it needs facts from below:** whether a tool reads, writes, deletes, sends or reaches the network. Tool-specific facts come from the tool (rune-tools) through the generic form kit defines (rune-kit). Without them, Studio treats the tool as unknown and asks the user every time.
3. **Stop must work everywhere.** When the user presses Stop, Studio cancels in-flight work. Each layer is asked to actually stop (not just stop reporting).
4. **Background work must yield.** Studio will run reflection while idle. It must never delay a real task.
5. **Models are rune-infer's only.** Studio calls rune-infer for anything model-based, including OCR and vision. It does not look for model features in kit or tools.
6. **UI is data.** Tools carry JSONL blueprints; Studio renders them from a fixed catalog of components in its design system. Actions from a widget that trigger a tool call are ordinary tool calls and go through the safety gate.
7. **Compatibility:** each project announces breaking changes in release notes. Studio keeps a conformance script (`scripts/wire-audit.mjs`, checks what Studio sends against a live rune-infer) and offers to run it on request; equivalent checks for kit are proposed below.
8. **Shared test:** Studio's field test and evaluation suite (15 fixed desktop tasks, success and time recorded) are how Studio measures whether a change in any project helped or hurt.
9. **Models are interchangeable.** Studio asks rune-infer what the loaded or available models can do and keeps its own per-model profile; a model change never needs a Studio release. Sensitive data (finance, health, mind) is only ever sent to a **local** model; rune-infer must say which models are local and which are cloud.

## 3. Phases and where this project is needed

| Phase   | Studio work                                      | Needed from the other projects                                                                                         |
| ------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| A (now) | M1 model profiles and roles; L0 life foundations | rune-infer: structured capabilities and locality, cancel, honest `usage`, residency visibility, a model support matrix |
| B       | R6c post-mortems, R7 notes                       | rune-infer: a way for background calls to yield without causing a model swap                                           |
| C       | K1: kit client                                   | rune-kit and rune-tools: JSON output, risk facts, cancel and timeouts, a curated starter set                           |
| D       | K2: A2UI surfaces                                | rune-kit: carry blueprints. rune-tools: a blueprint for a pilot tool. Studio: renderer and catalog                     |
| E       | R8, R9 universal                                 | Stable contracts, conformance tests, a second surface (browser through kit)                                            |

Studio's need-by points are stated per request below. Studio does not impose a date; tell Studio what is realistic and Studio will adjust order.

## 4. How Studio will use rune-tools

Studio never talks to a plugin directly. Through rune-kit (planned milestone K1) the model in Studio will be offered some of your tools, each call passing through Studio'"'"'s safety gate: a risky call waits for the user'"'"'s Yes, and an **unknown tool defaults to ask every time**. Studio starts with a curated **read-only** set and widens only after its evaluation shows the tools help. So what Studio needs from you is mostly **facts about each tool** and, later, **a UI blueprint per tool**, and confidence that the starter set is stable on Windows.

## 5. What Studio understands about rune-tools today (read earlier, **NOT CERTAIN** now; correct it)

14 plugins: five WASM (filesystem, fetch, memory, sequential-thinking, time) and nine native (git, browser, email, audio, video, image, slides, figma, one private email-template plugin). The browser plugin needs `agent-browser` 0.38 or newer and has a screenshot tool. There is no desktop-screen, OCR or vision plugin. The memory plugin stores a JSON knowledge graph. The README and AGENTS file list plugins that do not exist (print, scan, ssh, game-engine).

## 6. Requests

### TOOL-1 (Must, now buildable) Each tool declares what it can do, in rune-kit's form

- **Division of rules (decided by the owner):** a rule specific to one tool belongs to that tool in rune-tools. Generic rules (the vocabulary, the defaults, enforcement) belong to rune-kit, which has now **built** its side (rune-kit report, 2026-10-21; verify against your copy of rune-kit before relying on it).
- **What rune-kit now provides (as reported, check it):** a plugin declares per-tool facts in its `plugin.toml` under `[tools.<tool_name>]` with optional booleans `read_only`, `destructive`, `idempotent`, `open_world` and an optional `description`. kit merges them into the tool's definition and returns them as `annotations` in MCP `tools/list`. A tool that declares nothing has **no annotations**, and Studio then asks the user every time. `rune run --read-only` (and `--allow-tool <name>`) show and allow **only tools declared `read_only = true`**: an undeclared tool is invisible in read-only mode and a forced call to it is refused. kit also serves a rules resource `rune://<namespace>/rules` with the plugin's declared capabilities and tool rules.
- **Meaning to use (state it in your docs):** `read_only = true`: no side effects anywhere. `destructive = true`: can delete, overwrite or irreversibly change something. `open_world = true`: reaches outside the machine or outside the plugin's own data (network, other accounts, other people). `idempotent = true`: repeating the call with the same arguments has no further effect. If unsure, **leave it undeclared or set the safer value**; never set `read_only = true` for a tool that can write.
- **Ask:** declare every tool of every plugin, in this order: **filesystem, git, time** first (they are Studio's first read-only set and its evidence sources), then fetch, sequential-thinking and memory, then the rest. Also the tool-specific limits only the tool knows (for example the folder a filesystem tool is rooted at, the hosts fetch may call), in the plugin's capabilities as rune-kit defines them.
- **Check:** `rune run --all --read-only` lists exactly the tools you declared read-only, and nothing that can write; `rune run --all` lists annotations for every declared tool; a forced call to a write tool in read-only mode is refused. Studio will compare your declarations with its own risk table and report mismatches.
- **Not asked:** UI blueprints now (TOOL-9 waits for rune-kit 0.4.0 and Studio's catalog).

### TOOL-2 (Should) Stability statement for the starter set

- **Ask:** tell Studio which plugins you consider stable on **Windows** today and tested. Studio'"'"'s proposed first read-only set (Studio decides, you advise): filesystem reads, time, and possibly fetch and sequential-thinking. Which of these have tests, and which have known problems on Windows (paths, line endings, long paths, permissions)?

### TOOL-10 (Question) Evidence sources for the life track

Studio's life track will, with the user's consent, read **files changed in chosen folders, git history and time** as evidence of what the user did (read-only, through kit). Please confirm for the filesystem, git and time plugins: which of their tools are strictly read-only and declared so (TOOL-1), whether git history can be read without a working-tree write, and any Windows caveats. Also tell Studio whether a **calendar** plugin or a **tabular/CSV read** helper exists or is planned (Studio does not know of one; it can parse CSV itself, so neither is required).

### TOOL-3 (Resolved) OCR and vision

- **Decided (owner):** OCR and vision are model work and are done by rune-infer. rune-tools will not host a screen OCR or vision plugin, and Studio will not look for one here. No action; the question is closed. (If a **non-model** desktop utility is ever wanted as a tool, such as listing windows, that would be rune-tools' to decide; Studio does not need one now, it has its own.)

### TOOL-4 (Should) Output contracts

- **Why:** Studio puts tool results into the model'"'"'s prompt and its record, so size and shape matter.
- **Ask:** per tool (or per plugin), documented limits: maximum result size and what happens past it (truncation? error?), how binary data such as images is returned, the shape of errors, and typical and maximum run time.

### TOOL-5 (Should) The browser plugin as a second surface

- **Why:** it is Studio'"'"'s candidate for a DOM-level browser surface (clicking by page element instead of by pixel), which would fix Studio'"'"'s weakest case, small targets on web pages.
- **Ask:** readiness on Windows (install steps for `agent-browser`, how the plugin reports a missing dependency), what a screenshot result looks like, and the tools you consider stable.

### TOOL-6 (Should) Accurate documentation

Remove or mark the plugins listed in the README and AGENTS file that do not exist, so Studio and users do not plan around them.

### TOOL-7 (Question) The memory plugin

Studio may later offer to copy accepted notes about apps into `rune-memory` so other agents can share them. Studio is **not** committing to this (**NOT CERTAIN** it is wanted). Question: is the knowledge-graph format meant for this kind of data, and is there a stable way to write to it?

### TOOL-8 (Process) Versioning

Plugin versions Studio can pin, and release notes that call out changes to tool names, arguments and results (the `<ns>__<tool>` names are part of what the model sees).

### TOOL-9 (Must for K2) A UI blueprint per tool

- **Decided (owner):** each tool holds its own JSONL blueprint for UI widgets; Rune Studio renders it with its own design system. It is data, not code.
- **What a blueprint is (Studio's proposal, for the PM to react to):** a small JSONL file shipped with the plugin that describes how to show a tool's result (and, if useful, a tool's input form) using only components from a **catalog that Studio publishes** (text, heading, list, table, key-value, image, progress, button, text field, chips, and similar; the final list and version are Studio's, to be published before K2 starts). Fields are bound to the tool's result by path. It contains no scripts, no markup, no styles, no colours (Studio's design system supplies the look) and no URLs except those the plugin already declares.
- **Format basis:** Studio follows the open A2UI protocol (Apache 2.0, created by Google; v0.8 stable and v0.9 current as found on 2026-10-20; JSON/JSONL, component catalog). **NOT CERTAIN** which version Studio will pin and which parts it will support; the first version of the catalog will be small. The PM may read the spec at a2ui.org.
- **Ask:** (a) agree the idea; (b) pick one or two **pilot tools** (Studio suggests a filesystem listing or read, and the time tool, because they are simple and low-risk) and write a blueprint for them once Studio publishes the catalog; (c) tell Studio where the blueprint file lives in a plugin so kit can carry it (KIT-11).
- **Studio offers:** a catalog document, a validator command that checks a blueprint against the catalog, and a preview, so authors can see their widget in Studio's style without building Studio.
- **Not asked:** no UI code, no framework, no styling.
- **Check:** Studio renders a pilot blueprint against a real tool result.

## Response table (fill in and return)

| ID  | Accept / Counter / Decline / Question | Owner | Target release | Notes or answer |
| --- | ------------------------------------- | ----- | -------------- | --------------- |
|     |                                       |       |                |                 |

Also tell Studio: (a) the one contact for Studio-facing questions, (b) how you announce breaking changes, (c) anything in section 1 or 2 that is wrong about your project.