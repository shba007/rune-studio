# Rune ecosystem: overall plan (for all project managers)

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

## 4. Cross-project requests at a glance

| ID             | To         | One line                                                                                        | Need-by                                      |
| -------------- | ---------- | ----------------------------------------------------------------------------------------------- | -------------------------------------------- |
| INF-1          | rune-infer | A cancelled request must stop generating                                                        | Now                                          |
| INF-3          | rune-infer | A way for background calls to yield to real tasks                                               | Before R6c ships                             |
| INF-7          | rune-infer | Structured capabilities per model (modalities, tools, thinking control, limits, engine)         | M1                                           |
| INF-15         | rune-infer | Each model is marked local or cloud; echoed on responses                                        | L0                                           |
| INF-16         | rune-infer | Interchangeable models: served-model identity, optional roles, clear errors                     | M1                                           |
| INF-17         | rune-infer | Support matrix for MiniCPM-o 4.5, Bonsai 2 27B and others                                       | M1                                           |
| INF-4          | rune-infer | Make "fits in VRAM stays, else swaps" visible and avoidable (suggestions inside)                | Before fast checks or reflection are enabled |
| INF-5          | rune-infer | A reliable speech-to-text route for voice                                                       | Voice quality                                |
| INF-10         | rune-infer | OCR, vision and box routes documented (confirmed as rune-infer's)                               | Before Studio uses them                      |
| KIT-1 to KIT-3 | rune-kit   | JSON output, generic rules and tool facts, cancel and timeouts                                  | Before K1 starts                             |
| KIT-11         | rune-kit   | Carry each tool's UI blueprint                                                                  | Before K2 starts                             |
| KIT-13         | rune-kit   | A read-only or allowlist profile                                                                | K1, L4                                       |
| TOOL-10        | rune-tools | Read-only facts for filesystem, git and time as life evidence                                   | L4                                           |
| TOOL-1         | rune-tools | Each tool declares what it can do, in kit's vocabulary                                          | Before K1 starts                             |
| TOOL-9         | rune-tools | A UI blueprint for each tool; two pilots first                                                  | Before K2 starts                             |
| INF-14         | rune-infer | A cleaner TUI following the Rune design language (spec attached); library exposes what it shows | At the PM's pace                             |
| KIT-12         | rune-kit   | When a TUI is built, it follows the Rune design language; library exposes what it would show    | When a TUI is planned                        |

Full lists with evidence and checks are in the three project documents.

## 5. Decisions

**Made by the owner (2026-10-20):**

1. **Model work is rune-infer's**: OCR, vision, speech, decisions, grounding. Kit and tools host no models.
2. **Non-model tools are rune-kit's to manage and rune-tools' to implement.** Kit is to tools what rune-infer is to models.
3. **Rules:** tool-specific rules live in rune-tools; generic rules live in rune-kit.
4. **Residency:** a model that fits in VRAM stays loaded, otherwise models swap.
5. **A2UI:** tools hold their own JSONL UI blueprints; Studio renders them with its design system; no UI code is generated at runtime.
   6a. **Models are interchangeable** (MiniCPM-o 4.5, Bonsai 2 27B and others; hot swap in rune-infer). Studio keeps per-model profiles and per-role choices (M1). Sensitive life data only goes to local models (INF-15).
6. **Interfaces:** the rune-infer TUI becomes cleaner and every Rune TUI, including a future rune-kit one, follows Rune Studio's design principle (dot interface, Nothing-phone style). References: `rune-design-language.md`, `rune-infer-tui-spec.md`.

**Still open:** 0. **Libraries:** rune-infer and rune-kit each have a library. Studio plans to use rune-infer over HTTP and rune-kit through the `rune` CLI and MCP over stdio. Question for both PMs: is there a stable library API that is meant for outside programs, and would you prefer Studio to use it? Studio has no preference beyond stability and being able to cancel work.

1. **Making swaps workable.** The owner is not sure how; suggestions are in INF-4. The rune-infer PM chooses.
2. **How kit carries blueprints** (KIT-11) and what the generic declaration vocabulary is (KIT-2). Two PMs, one answer.
3. **Skills:** how an agent consumes rune-kit skills (markdown). Studio has no skill concept and does not want to invent one. **NOT CERTAIN** whether kit intends to serve them over MCP (KIT-8).
4. **A2UI version and catalog:** which A2UI version Studio pins and which components are in the first catalog. Studio decides and publishes before K2; tools should not write blueprints before then.
5. **Release rhythm:** how often each project releases, and how breaking changes are announced.

## 5b. A2UI: how the UI is made without runtime UI code

The owner's constraint: Vue code must not be written at runtime, because a model producing UI code costs inference time (a normal Studio decision call is about 4.5 s with thinking off) and carries risk. Studio's approach, in order of preference:

1. **Static blueprint plus a generic renderer (recommended).** The tool ships a JSONL blueprint written once, by a person, at development time. Studio's renderer is a fixed set of Vue components (the catalog) that follows the blueprint and binds data from the tool's result. No model is involved, so no inference time is added. The extra cost is rendering only (**not measured yet**).
2. **Cached by tool and version.** Studio keeps the parsed blueprint, keyed by tool name and blueprint version, and re-parses only when kit reports a change.
3. **Precompiled templates for Studio's own surfaces.** Studio's built-in cards (analysis findings, the review card) stay ordinary Vue components, not blueprints. Only third-party tool UI goes through the renderer.
4. **Model-authored UI as an exception, off by default.** For an ad-hoc surface (for example a screen-analysis result), the model could emit a very small JSONL message; that costs generation time and needs schema-constrained output (INF-6). It would be measured before it is ever enabled.

**Safety rules for the renderer (Studio's side):** only components in the catalog; no scripts, markup, styles or colours from a blueprint; no external URLs unless the plugin declares them; size and nesting limits; a malformed blueprint is dropped and the tool's plain text result is shown instead; a widget action that triggers a tool call is a normal tool call and goes through the safety gate; the look always comes from Studio's design system.

**Where it appears in Studio:** as compact cards above the orb, in the same style as the screen-analysis cards (small, dark, dot-matrix, no clutter).

## 6. Risks (Studio's view)

- Cancel does not propagate: Stop appears to work in Studio but the server keeps working and the next task is delayed for over a minute (observed: 75.7 s).
- Unknown tools default to "ask the user every time", which makes kit tools annoying; without risk facts the integration is safe but not pleasant.
- Model swaps: any second-model call (decisions, speech, verifier, OCR) can cost 7 to 16 seconds if the models do not fit together in VRAM.
- Contracts drift silently without a conformance check.
- A2UI: the spec is young (v0.9 is current), there is no Vue renderer that Studio knows of, and a blueprint format shared by many tool authors is hard to change once published.

## 6b. Studio's own handoffs (given directly to Studio's agents)

- **M1 Model profiles and roles** (`rune-studio-M1-model-profiles.md`): Studio stops assuming one model.
- **L0 Life foundations** (`rune-studio-L0-life-foundations.md`): consent, store, evidence record, local-only enforcement, wellbeing basics.
  Both are independent of each other except that L0's local-only check uses M1's locality field when it exists.

## 7. What Studio will not do

Build its own model server, MCP server host, plugin manager or skill store; parse human-readable tables from other tools' CLIs; trust a tool's own claim of being safe; generate UI code with a model at runtime.

## Response table (fill in and return)

| ID  | Accept / Counter / Decline / Question | Owner | Target release | Notes or answer |
| --- | ------------------------------------- | ----- | -------------- | --------------- |
|     |                                       |       |                |                 |

Also tell Studio: (a) the one contact for Studio-facing questions, (b) how you announce breaking changes, (c) anything in section 1 or 2 that is wrong about your project.