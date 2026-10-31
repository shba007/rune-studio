# Handoff to the rune-kit project manager

## Agent brief (read first)

You are an AI coding agent working **inside the rune-kit repository**. The owner gave you this document. It was written by the Rune Studio orchestrator **from outside your repository**: what it says about rune-kit comes from Studio's observation (a field test and earlier reading) and **may be wrong or out of date**. Your first job is to check every statement about rune-kit against your code and say where it is wrong.

How to work:

1. Read this whole document and your repository's own contributing rules, branch rules and test commands. **Follow your repository's rules**, not any in this document. Do not push to a protected or release branch, and do not cut a release.
2. For every request below, find the current state in your code (exists, partly, missing) and write it down before you change anything.
3. **Build** the items in the build list (KIT-1, KIT-2, KIT-3 and KIT-13; the others are reports or Questions). For the others, report feasibility, and implement one only if it is small and clearly safe; otherwise leave it for the owner. Items marked Question or Process are answered in your report, not built.
4. Where a request offers options, choose the simplest that works, state the choice and why, and keep it compatible with existing clients (add, do not break; announce anything that changes behaviour).
5. Add tests for what you build. Run your repository's tests, lints and formatters and report the numbers before and after. **Never claim a behaviour you did not run.**
6. Mark anything you could not verify **NOT CERTAIN**. Do not invent facts about models, protocols or other projects.
7. Do not change the goals or scope of rune-kit beyond this document. If a request conflicts with your code or design, say so and propose the smallest alternative.
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

## 4. What Studio plans to do with rune-kit (milestone K1)

- Studio will spawn `rune run --all` as a child process and act as an **MCP client over stdio**: `initialize`, `tools/list`, `tools/call`, and resources and prompts where useful.
- The model in Studio is offered the desktop actions plus selected kit tools (namespaced `<ns>__<tool>`).
- **Every kit tool call passes through Studio'"'"'s safety gate before it runs.** A risky tool waits for the user'"'"'s Yes; an **unknown tool defaults to ask**.
- Studio records every call (kit has no logs of its own, as read earlier) and cancels in-flight calls on Stop.
- Installing, updating and removing kit, plugins and skills **stays on the `rune` CLI**. Studio will not parse human-readable tables.
- Rollout: Studio starts with a curated read-only set and widens it only after its evaluation shows the tools help.
- Studio does not start K1 work until KIT-1, KIT-2 and KIT-3 are answered.
- **K2 (after K1): A2UI surfaces.** Each tool holds its own JSONL UI blueprint; Studio renders it with its own design system from a fixed catalog of components. Kit is the carrier (KIT-11). The UI is data; no UI code is generated or run at runtime.
- Kit is the manager of all **non-model** tools. Studio does not look for model features (OCR, vision, speech) in kit; those are rune-infer's.

## 5. What Studio understands about rune-kit today (read earlier, **NOT CERTAIN** now; correct it)

One `rune` binary with commands `run`, `install`, `uninstall`, `list`, `available`, `search`, `update`, `self-update`. Table output only. MCP over stdio, protocol `2024-11-05`. Plugins are WASM (sandboxed through Extism) or native child processes over stdio JSON-RPC, with capabilities declared in each plugin manifest. A native sidecar call is serialised per process. No HTTP transport, no logs beyond stderr.

## 6. Requests

### KIT-1 (Must) Machine-readable CLI

- **Ask:** `--json` (or an equivalent) for `list`, `available`, `search`, `install`, `update --check`; stable exit codes; non-interactive installs and updates; a `rune --version` Studio can parse.
- **Why:** Studio shows install state and offers updates but refuses to scrape tables.
- **Check:** Studio parses the JSON in a test against a fixture.

### KIT-2 (Must) Generic rules, and tool facts at call time

- **Division of rules (decided by the owner):** rule that is specific to one tool lives in that tool (rune-tools). Rules that are generic across tools live in **rune-kit**. Kit is the home of: the **vocabulary** a tool declares in (for example read-only / writes / destructive / sends externally / uses the network / runs processes, the form of declared hosts, folders and binaries); **defaults** for a tool that declares nothing; **validation** of declarations at install; **enforcement** (what is actually blocked for WASM and for native plugins); and **how the facts are passed on** to a client.
- **Why Studio needs it:** the safety gate must classify a call before it runs. Without facts every kit call is "unknown, ask the user".
- **Ask:** surface per-tool facts through MCP. To Studio's knowledge, MCP versions newer than `2024-11-05` have tool annotations (read-only, destructive, idempotent, open-world hints); the PM should confirm. If not, expose the declared capabilities another way (an MCP resource, or the CLI). Tell Studio which form you choose, the vocabulary, and the default for an undeclared tool.
- **Check:** Studio lists tools with their facts and maps them to its policy table; a tool with no facts stays "ask".
- **Dependency:** the tool-side declarations are TOOL-1 in the rune-tools handoff.

### KIT-3 (Must) Cancel, timeouts and failure behaviour

- **Why:** Stop must stop. A hung or crashed plugin must not freeze the agent.
- **Ask:** honour the MCP cancellation notification and stop the work (or say what cannot be stopped); a per-call timeout and what the error looks like; what happens when a plugin process crashes or hangs (restart? error to the client? other plugins unaffected?); document the effect of serialised native sidecar calls (a long call blocks others).
- **Check:** Studio cancels a long call and times how fast the child reports back.

### KIT-4 (Should) Protocol version and progress

- **Ask:** the protocol version policy (target version, whether tool `annotations`, progress notifications and `list_changed` are supported or planned).

### KIT-5 (Should) Security defaults

- **Why:** an empty `network_hosts` combined with an empty allowed-hosts list allows any host, and native plugins have no sandbox (as read earlier). Studio will treat every kit tool as untrusted regardless, but users will assume "installed through kit" means "contained".
- **Ask:** a documented statement of the default and of what is enforced for WASM and for native plugins; whether this will change.

### KIT-6 (Question) Concurrency and process model

Does a second client starting `rune run --all` (for example another agent on the same PC) conflict with Studio'"'"'s? Start-up time of `run --all` with N plugins? What happens if a plugin fails to install or start: do the others still serve?

### KIT-7 (Should) Distribution on Windows

How does Studio find `rune` (PATH, an installer, a known folder)? Behaviour of `self-update` while Studio has the gateway running. Confirmation that Windows is a supported platform for the binary and the plugins Studio will start with.

### KIT-8 (Question) Skills

Studio has no skill concept and does not want to invent one. How is an agent expected to consume installed skills (markdown in `skills/<name>/`)? Is serving them as MCP prompts or resources planned? **NOT CERTAIN**; the answer decides whether Studio does anything with skills at all.

### KIT-9 (Nice) Observability

Structured logs or events for tool calls (timing, result size, error) with a correlation id Studio can attach to its own record.

### KIT-10 (Process) Compatibility

Release notes that call out changes to the CLI output, MCP behaviour and plugin lifecycle. Studio offers to write a small conformance test (spawn `rune`, list tools, call a harmless tool, cancel one) for you to run in your CI.

### KIT-11 (Must for K2) Carry each tool's UI blueprint

- **Decided (owner):** each tool holds its own JSONL blueprint for UI widgets. Kit is the manager, so it carries them.
- **Ask:** (a) how blueprints are discovered by a client: Studio suggests an MCP resource per tool or plugin (a stable URI per tool, listed in `resources/list`, with a change notification) so no new protocol is needed; the PM decides; (b) validation at install: well-formed JSONL, within a size limit, and only using components from the catalog version it declares; (c) versioning: the blueprint declares the catalog version it targets; kit reports it; (d) how a widget action maps back to a tool call (see below); (e) hot update: installing or updating a plugin changes its blueprint and clients are told.
- **Data and actions (Studio's proposal, for the PM to react to):** a blueprint refers to data by path (for example into the tool's result), so the blueprint is static and only the data changes per call. A widget action (for example a button) names a tool and arguments; the client sends it as a normal `tools/call`. Studio's safety gate applies to that call like to any other.
- **Not asked:** kit does not render anything and does not call a model to make UI.
- **Check:** Studio reads the blueprint of one pilot tool through kit and renders it against a real result.

### KIT-13 (Must) A read-only profile for evidence and curated sets

- **Why:** Studio's life track reads files, git history and time as **evidence** about the user's work, and Studio's K1 starts with a curated read-only tool set. Studio's own gate decides risk, but a kit-level guarantee that **only read tools are exposed** is a second, independent safety layer.
- **Ask:** a way to start the gateway with an **allowlist or a read-only profile** (for example only named tools, or only tools declared read-only, see KIT-2), so a client that asks for read-only never sees a write tool, and a write attempt is refused by kit, not just by the client.
- **Check:** Studio starts kit in read-only mode, lists tools (no write tools), and a forced call to a write tool is refused.

### KIT-12 (Should, when a TUI is planned) A future TUI, and a library that serves it

- **Decided (owner):** rune-kit has **no TUI yet**. When one is built, it follows the design principle of Rune Studio (dot interface, Nothing-phone style). See `rune-design-language.md`. The rune-infer TUI spec (`rune-infer-tui-spec.md`) is a worked example of applying it; the same rules (now before history, hide idle things behind one line, one accent, dots for state and quantity, the word as well as the colour) apply.
- **Suggested starting point for the PM to adapt (illustrative, not a requirement):**

```
 ·  RUNE KIT 0.x        ● READY      3 PLUGINS RUNNING      UP 00:42:10

 PLUGINS     12 INSTALLED · 2 UPDATES
 ●  filesystem        0.4.1     WASM      3 TOOLS      ●○ UPDATE
 ●  fetch             0.3.0     WASM      1 TOOL
 !  browser           0.2.0     NATIVE    FAILED · agent-browser MISSING
    + 9 MORE · IDLE                                             P  LIST ALL

 NOW         filesystem__read_text_file     RUNNING  00:01      ⠋

 INSTALLING  rune-git        ●●●●●●····  60 %

 TAB  LOGS      /  FILTER      ?  HELP      Q  QUIT
```

- **Ask:** (a) design the TUI to the language when it is planned; Studio reviews screenshots on request; (b) design the **library first** so it exposes what a TUI (and Studio) would show: installed plugins and their state, update availability, install progress, running tool calls, failures with reasons, as structured data with cancellation (this is the same need as KIT-1 and KIT-3); (c) tell Studio whether the library (`rune-kit-core`) is meant to be used by outside programs and how stable its API is. Studio plans to use the `rune` CLI and MCP over stdio, and would consider the library if it gives structured results and cancellation more directly.
- **Not asked:** what the TUI does, or whether to build one.

## Response table (fill in and return)

| ID  | Accept / Counter / Decline / Question | Owner | Target release | Notes or answer |
| --- | ------------------------------------- | ----- | -------------- | --------------- |
|     |                                       |       |                |                 |

Also tell Studio: (a) the one contact for Studio-facing questions, (b) how you announce breaking changes, (c) anything in section 1 or 2 that is wrong about your project.