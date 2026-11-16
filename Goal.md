# Rune Studio — Goal

> This file states direction, not status. What works today is in `README.md` and `TODO.md`, and the
> code is the source of truth over any document. Decisions recorded here were
> made by the owner on 2026-11-16.

## The whole goal

Rune is a local, Jarvis-like AI harness. You talk or type, or it proposes work
itself; it does the work on your own devices with your own models, tools and
memory; it shows the real result of every step while it works; and it asks
before anything risky.

### Principles

1. **Local first, with a fixed fallback order.** A model runs on the device in
   use when it fits. If it does not fit, it runs on the user's own PC. If it
   does not fit there, it runs in the cloud. If all three fail for that model,
   the next equivalent model is tried in the same order. A cloud-only model
   goes straight to the cloud. Cloud is optional in general and is the expected
   path only for devices too weak to run models (laptop, phone, later IoT).
2. **Strict ownership.** Each concern has exactly one owner (table below).
3. **Three kinds of step, one owner each.** A model call belongs to Infer, a
   tool call to Kit, a screen action to Studio. A tool or model call is
   preferred; a UI Automation element comes next; pixels are last.
4. **Show the thing, not a description.** Every step shows what it actually
   produced (the email text, the table rows, the photos) and decisions are made
   on that content. The view is generated automatically from the schema the
   owner publishes: Kit for tools, Infer for model operations. No model
   generates a layout, and screen captures are not used as a substitute view.
5. **Live and replayable.** Steps appear as they start. The same event stream
   drives the live view and playback. The pill shows a glance; a workbench
   window shows and decides.
6. **Decisions are choices.** Several options, one suggested, each showing its
   outcome on the real content. Not a bare yes or no.
7. **A model per step.** Studio chooses which model a step needs, by the step's
   difficulty and the speed required. Infer decides where that model runs,
   following principle 1.
8. **Sub-agents when the goal needs them.** The plan shown to the user states
   whether one agent or an orchestrator with several sub-agents will do the
   work. Simple requests use one agent. When sub-agents are used they work in
   parallel wherever their work does not conflict.
9. **Autonomous by proposal.** Rune looks for useful work without being asked,
   by observing through tools (inbox, calendar, files), never by watching the
   screen continuously. By default it only suggests. Preparing work for
   approval is opt-in per area. Content from outside never raises what it may
   do unasked.
10. **Ask before assuming.** When a request lacks something needed to plan, or
    contradicts itself, Rune asks the user directly before planning: few
    questions, each with options and a suggested answer. It does not ask what
    it can look up or safely default, and it does not plan on a guess.
11. **Plans can grow.** An approved plan is a starting point. While working, a
    step may be added when a need is discovered, such as looking something up,
    fetching a file or producing something with a model. Added steps are shown
    as added and pass the same safety gate as planned ones.
12. **Shared materials with provenance.** Everything gathered or produced for a
    goal is kept in one place that every step and sub-agent can reuse, so the
    same item is not fetched or produced twice. Each item records where it came
    from: its source and licence if fetched, the model if generated. Claims
    taken from outside carry their source. Requests that leave the machine,
    such as a web search, must not contain sensitive data.
13. **Drafts, not restarts.** A result is kept as parts that depend on each
    other. The user can give feedback on any part; only what depends on that
    part is redone, and the outcome is a new draft. Earlier drafts are kept for
    comparison and return.
14. **Reuse before building.** Existing engines, MCP servers, research and
    projects are preferred over new code.
15. **Small cycles.** Changes ship as small, deployable steps ordered by the
    80/20 rule.
16. **Safe by default.** Risky actions need confirmation, everything is
    audited, and sensitive data classes stay on the user's own hardware.
17. **Scope for now.** Windows, Linux and Android. Both a personal tool and a
    public product.
18. **A non-goal is not a removal order.** Code that falls outside these goals
    stays as it is. Nothing is removed, disabled or rewritten because it is a
    non-goal unless the owner explicitly approves that removal.

### Ownership

| Repo            | Owns                                                                                                                                                                    |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **rune-studio** | The agent runtime and orchestrator, the user interface, the safety gate, screen actions, voice, the run trace, and the bridge that lets outside agents drive the screen |
| **rune-infer**  | Every model and engine, including specialist operations (OCR, detection, generation, speech), and where a model runs: device, home PC or cloud                          |
| **rune-kit**    | Every MCP server the harness uses (first-party plugins, third-party stdio and HTTP servers), skills, the sandbox, and shared external binaries such as ffmpeg           |
| **rune-tools**  | First-party plugins for Kit                                                                                                                                             |
| **rune-lake**   | Memory: the media catalog, long-term memory and the encrypted life store                                                                                                |
| **rune-agent**  | Sub-agents (personalities) and their workflows: a prompt, skills, MCP servers and a workflow each                                                                       |
| **rune-model**  | The model weight store, nothing more                                                                                                                                    |

## This repo's goal

Studio is the harness itself: the place the user meets Rune, and the runtime
that plans, acts, orchestrates and reports. It asks Infer for every model call
and Kit for every tool call, performs screen actions itself, and reads and
writes memory through Lake.

### Current goals

1. **Workbench and live timeline.** Show each step as it starts, with what it
   produced, rendered from the schema published by Kit or Infer. The step
   events (`step_started`, `step_finished`, `plan_updated`) are already emitted
   live; today the timeline is only rendered in after-the-fact views.
2. **Decisions on content.** Replace yes/no confirmation with several options,
   one suggested, decided on the real content in the workbench.
3. **Three step kinds.** Offer the agent Infer's model operations and Kit's
   tools directly, and keep screen actions as the last resort. Let background
   work use write tools behind the safety gate.
4. **Model per step.** Tag each planned step with what it needs and map that to
   a model id from Infer's catalog. Group steps that share a model, because
   switching models on one GPU costs seconds.
5. **Plan decides the team.** The plan shown first states whether one agent or
   an orchestrator with sub-agents runs the goal. The orchestrator runs
   sub-agents in parallel where their work does not conflict, and the workbench
   shows each sub-agent's work live and in playback.
6. **Conversation.** A plain conversational turn alongside goal, task and
   step, with a spoken reply through Infer.
7. **Proposals without being asked.** A scheduled pass over standing goals and
   read-only tools that puts a small number of proposals in the decision inbox.
8. **Runtime as a service, inside Studio.** The agent loop, board and scheduler
   run in a headless Studio process; the desktop window, a phone and a headless
   Linux box are clients. Home network first, a tunnel later.
9. **Only the embedded Infer.** Studio talks to its own Infer and nothing else
   for models; reaching another PC or the cloud is Infer's job.
10. **Playground as the test bench.** It checks models, tools and flows in
    isolation without running the whole app. It gains a Tools tab (form from a
    tool's input schema, result from its output schema) and tool steps in flows.
    Bench runs write no memory and stay out of task history.
11. **Hand over what others own.** The life store moves to Lake; personalities
    and workflows move to rune-agent; model facts come from Infer.
12. **Clarify step before planning.** Detect missing or conflicting details in
    a request and ask about them first. Intake already flags whether a request
    is a goal, task or step and has a clarify prompt; this extends it to the
    content of the request.
13. **Growing plans.** The orchestrator admits steps added during a run, marks
    them as added in the timeline, and applies the safety gate to them.
14. **Dependency record.** Record each step's inputs and outputs and the links
    between them, so that feedback on one part redoes only what depends on it.
    The trace records steps today, but not these links.
15. **Feedback and drafts in the workbench.** The user can comment on any part
    of a result; Rune produces a new draft, keeps earlier ones, and shows which
    parts were reused and which were redone.

### Not goals

- Running models or engines (Infer) or the MCP servers the harness uses (Kit).
- Owning memory or life data once Lake can hold them. The run trace stays here;
  summaries go to Lake.
- Model-generated layouts, or screen captures used as a substitute for a view.
- Watching the screen continuously to find work.
- Connecting directly to a remote or cloud model server.
- New Portfolio features inside Studio; the flow is to become a sub-agent.
- macOS, until the scoped platforms work.

Per principle 18, existing code in these areas is not to be removed without the
owner's explicit permission.

### Where the code is today

- A Windows screen-control agent: screenshot plus UI Automation, one action per
  model turn, with a safety gate, a stuck detector and an optional verifier.
- A task board that plans a request into tasks and steps; background tasks may
  use only read-only Kit tools.
- Goal intake, schedules and alarms, the Playground, a Portfolio flow, and the
  outside-agent bridge.
- Infer and Kit embedded in-process, pinned by git revision. Studio can also
  connect directly to other model servers today.
- No conversation turn, no spoken reply, no wake word, no memory read into the
  agent's prompt, no sub-agents. Reflection is a placeholder.

### Open questions

- How a phone on the home network authenticates to the runtime.
- Which parts of screen control make sense on Android, if any.
- How much of the task board must survive a restart.
- How the orchestrator detects that two sub-agents' work would conflict.