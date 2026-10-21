# Rune Life Harness: stable, deliverable milestones

For Shirsendu (owner). Written by the Rune Studio orchestrator on 2026-10-21. It turns the long-term goal (Rune as a **life harness**) into milestones that each end in something you can use. It builds on the existing plan (Studio R5 to R9 and K1/K2, and the rune-infer, rune-kit and rune-tools handoffs). It does not change the other projects' goals. Sizes (S, M, L) are my guesses, not commitments. Anything I could not verify is marked **NOT CERTAIN**.

## 1. What the old approach was, and why this one differs

The Solo Leveling design you pasted (an Obsidian vault, Python scripts, Gemma through Ollama, quests, XP, a nightly review) is a good first answer to "how do I stay consistent". Its weak point is the one you named: it **trusts what you type**. XP comes from you marking a quest 0 to 100%, and the "verification" is a random question that is logged, not used. A system like that rewards whoever games it, and the first person it fools is you.

The Rune life harness is built on different rules:

1. **Evidence, not belief.** Every claim in the system (a goal is on track, a habit is held, a skill improved, money was saved) carries where it came from and how strong that is. A claim that is only your word is stored as your word. Nothing is "known" because it was said.
2. **No assumptions.** Every plan lists the assumptions it rests on, each marked *untested*, *supported* or *refuted*. The system asks the smallest question that would settle an assumption instead of guessing. "I don't know yet" is a valid state.
3. **You can be wrong, and so can it.** People misremember, flatter themselves and hold wrong beliefs; so does a small local model. When claims and evidence disagree, the system asks a curious question ("the log shows no commits this week; what happened?"), never an accusation. Its own conclusions are labelled with confidence and can be challenged.
4. **Plans are hypotheses, and can change mid-session.** Each plan states what would show it is not working and when that will be checked. When the evidence says the trajectory will not reach the goal, the system says so, offers options, and revises the plan with a recorded reason, with your approval.
5. **Collaboration, not autopilot.** It does not live your life for you. It improves **you** (clearer thinking, better decisions, honest feedback) and **itself** (better advice, measured). What can be automated, it automates **after asking**, within a scope you approved, with a record and an undo.
6. **Your data stays yours.** Local by default, per-domain consent, retention and delete, and a hard rule that sensitive domains never go to a cloud model.

## 2. The core model (the same in every milestone)

**Entities.** *Vision* (why), *Goals* (long term), *Objectives* (measurable, time-boxed, each with a stated success evidence), *Actions/Quests* (daily and weekly items that trace to an objective), *Skills*, *Resources* (time, energy, money), *Assumptions*, *Plans* (versioned), *Reviews*, *Lessons* (what you learned, in your words).

**Evidence ladder.** Every record has a source and a grade:

| Grade | Meaning | Example |
| --- | --- | --- |
| E0 | Your word only | "I went to the gym" |
| E1 | Your word with a specific, checkable detail | "Back and biceps, 50 min, ended 7:10" |
| E2 | Observed by a tool or imported data you did not type | Commits in a repository, a file changed, a calendar block, a wearable export |
| E3 | Independent and hard to fake | A statement from the bank, a measured result over time |

Progress, status and (later) XP are computed from the grade, not from the claim. E0 counts, but little; the system says so. Over time the system tracks your **calibration** per domain (how often your claims match evidence). It is shown to you as a number you can improve, not as a judgement.

**Plans as hypotheses.** A plan stores: the goal, the assumptions, the expected evidence by date, a **review trigger** ("if fewer than 3 sessions by week 3, re-plan"), and its version history with reasons.

**Automation ladder.** A0 observe only. A1 suggest. A2 do it after asking each time. A3 do it inside a scope you approved (what, where, until when), always logged, with undo. A4 act without telling you first, only for narrow classes you chose. Anything that touches money, messages to other people or deletes data never goes past A2.

**Where it lives in the ecosystem.**

| Need | Provided by |
| --- | --- |
| The conversation, cards, dashboards, scheduled prompts, safety gate for actions, the record | Rune Studio |
| Thinking, summarising, structured answers, OCR, any model work | rune-infer (Gemma 4 locally) |
| Reading files, git history, time, importing data, later calendar or mail | rune-kit with rune-tools (K1) |
| Tool-specific UI (a finance import form, a chart) | A2UI blueprints (K2) |
| Your notes | Obsidian, as a readable view and an input (see decision 2) |

## 3. What carries over from the old design, and what changes

| Old idea | Verdict |
| --- | --- |
| Morning brief, evening debrief, weekly strategy | Kept: it is the daily loop (L3, L6) |
| 6 stats, ranks, seasons, quests | Kept as a skin, but computed from evidence (L7); built late so the loop is honest before it is fun |
| Strict structured logs | Kept, and strengthened: every record has source and grade |
| Rolling compression of history | Kept (needed for a small context window) |
| Each quest traces to a goal; orphan quests flagged | Kept (L2) |
| Hard cap of 5 daily quests, backlog | Kept (L3), capacity-aware |
| Random "describe what you did" checks | Replaced by evidence collection plus calibration (L4); no random quizzing |
| Consequence mapper | Kept, but labelled as a projection with assumptions and confidence, tested against what actually happened (L5, L9) |
| 40-question onboarding in one sitting | Replaced by an adaptive strategy conversation over several short sessions (L1) |
| Commitment statement | Kept, and checked against evidence the next day |
| Obsidian as source of truth | Changed: a database is the source of truth, Obsidian is a generated view plus an input (decision 2) |
| Finance as monthly "compound move" | Kept, but starts read-only from imported statements (L4); it never moves money |
| "Never accept shallow answers" | Changed: shallow answers are accepted but graded E0; forcing prose produces fluent nonsense |

## 4. Milestones

The platform track continues unchanged underneath: Studio R5 (identity, demo mode, idle shell), R6b (Flight Player), R6c (post-mortems and notes), R7 (notes in the prompt), R8 (trust), K1 (kit client), K2 (A2UI). The life track below uses them.

### L0 Foundations: consent, store, trust slice (M)
**Why first.** Sensitive data must not be collected before it can be protected. This pulls a **minimal slice of R8** forward.
**Deliverables**
- A separate life database (not the agent trace), with data classes: general, health, mind, finance. Per-class consent switch, retention, export, delete.
- The evidence record: every row has `source`, `grade`, `time`, and a link to what supports it. Append-only with an audit trail.
- The assumption register and the plan/version tables (empty until L1).
- **Local-only enforcement.** The life module refuses to send finance, health or mind data to a model that is not local. It needs to know whether the configured model is local (new request INF-15, section 7); until then it only sends to a base URL the owner marked local.
- Encryption at rest. The mechanism is **NOT CERTAIN** (Windows data protection through the OS account, or an encrypted database library); decided in the L0 spike.
- A basic wellbeing protocol: the system is not a clinician, gives no diagnosis, and for talk of self-harm it stops advising and shows fixed, human-written support information. Detection by a small local model is **NOT CERTAIN** to be reliable, so this is conservative and tested with a fixed phrase set; it is not a safety guarantee and says so.
**Exit:** create, read, export and delete work and are tested; a finance record cannot be sent to a non-local model (test); redaction removes secrets from logs; the owner reads and signs off the privacy statement.

### L1 Strategy session (M)
**Deliverable.** A guided conversation in a Studio window (the Rune design language: calm, compact, dots) that produces your starting point: vision, goals, current objectives, skills, a money snapshot, constraints (time, energy, health), and a list of **assumptions** with statuses.
- Adaptive, short sessions (about 15 minutes, resumable), at most one clarifying question at a time, with "skip" and "I don't know" always available. Not a 40-question form.
- Every answer is stored as E0 or E1 with its source; the system asks what evidence could confirm the important ones.
- Output you can edit: a goal tree, three horizons (this week, this quarter, this year), and a first plan stated as hypotheses with review triggers.
- Advisor quality spike: before building further, run the same session on **each candidate model** (Gemma 4, MiniCPM-o 4.5, Bonsai 2 27B and others; models are interchangeable, see milestone M1) with thinking on and off, read the results with the owner, and keep a short per-model note of what each cannot do (**NOT CERTAIN** how well it challenges you; this is the main risk to the whole track).
**Exit:** the owner completes the session for real; the result is editable and exportable to Obsidian; at least 10 assumptions are recorded with statuses; the spike report exists.

### L2 Objectives, progress and status (M)
**Deliverables**
- The objective model with a required *success evidence* statement ("done when: three commits and one demo recorded").
- Status computed from evidence (not set by hand): not started, moving, stalled, at risk, done, with the reason shown.
- A dashboard window in the dot style: per goal and per objective, a row of dots for progress, a word for status, and the evidence behind it one click away. A light "level" preview here is fine; the full game layer comes in L7.
- Orphan detection: an action that traces to no objective is flagged.
- Obsidian export (and re-import of edits, treated as E0 edits with a diff).
**Exit:** every objective has success evidence; status changes only when evidence changes; the owner can answer "why is this at risk?" from the screen.

### L3 The daily loop (M)
**Deliverables**
- **Morning (about 5 minutes):** at most five items from the backlog, capacity-aware (it learns how much you actually finish), one commitment you state, one question. Shown as cards above the orb or in the life window, started by a scheduled prompt (uses the idle/shell machinery from R5).
- **Evening (about 10 minutes):** for each item, how much was done and one line of detail; the system reopens the morning commitment first.
- Next-day structure built from the result.
- No automation yet; the loop is advice and structure.
**Exit:** the owner uses it for 14 days in a row (dogfood), the daily time is measured and stays under the stated budget, and adherence data exists.

### L4 Evidence collectors and calibration (L)
**Deliverables** (every source opt-in, individually switchable, with a "what is collected" screen)
- Imports: bank statement files (read-only), wearable or fitness exports, notes.
- Tool-based: files changed in chosen project folders and git history, through kit's filesystem and git tools (needs K1; until K1, Studio reads these itself).
- Time: calendar blocks if a source exists (**NOT CERTAIN**: no calendar plugin is known to exist).
- No screenshots and no continuous screen recording for life evidence. If activity sampling is wanted (which app, how long), it is a separate opt-in that records only the program name and duration, **off by default**.
- Claims matched to evidence automatically; mismatches go to a curious-question flow; the **calibration** score per domain is shown.
- Studio's own verifier (a model comparing a claim with a screenshot) made false contradictions in the field test (2 of 4 decisive Notepad verdicts). So an automatic mismatch is a prompt to ask, never a conclusion.
**Exit:** at least three sources working and removable; one claim goes end to end from "I did X" to "evidence found" to a graded record; a mismatch produces a question, not a penalty.

### L5 The advisor: questions, consequences, critical thinking (L)
**Deliverables**
- Socratic mode: it asks the question that would change the decision, states the opposing view, and says what evidence it relies on (ids shown).
- Consequence maps: best, likely and worst paths over 30, 90 and 365 days, each with its assumptions and a confidence, labelled as projection.
- Anti-flattery rules: it must not agree with a claim that evidence contradicts; it must say when it lacks evidence.
- A mindset archive: lessons you wrote, recalled when a similar decision comes up.
- **An advisor test suite** (fixed scenarios): a false belief the advisor must challenge, a missing-evidence case it must admit, a flattering prompt it must not echo, a crisis phrase it must not advise on. Pass levels set by you.
**Exit:** the suite passes at the agreed level on the local model; advice always cites evidence ids or says "no evidence"; you rate a week of advice as useful and non-flattering (**subjective; yours to judge**).

### L6 Trajectory review and mid-session re-planning (M)
**Deliverables**
- Weekly review (about 30 minutes): evidence against each plan's expected evidence and review triggers.
- **Divergence detection:** when a trigger fires, the system says the current trajectory is unlikely to reach the goal, shows the evidence, and offers two or three options (adjust effort, change the target, change the method, pause), each with consequences.
- **Re-plan at any time**, including in the middle of a conversation: a new plan version is stored with the reason and the evidence, you approve it, and you can roll back.
- Assumption register updated: supported or refuted, with what changed.
**Exit:** at least one real or replayed case where a review trigger fired and the plan was revised with a changelog; rollback works.

### L7 The game layer (M)
**Deliverables**
- Stats, ranks and seasons (a Solo Leveling skin: "Hunter", "System" as language only, no outside assets), a dot-matrix HUD in the Rune design.
- XP and stats **computed from evidence grade** (E0 small, E2 and above larger), diminishing returns, no points for restating the same claim.
- Rest is part of the game: planned rest days, no streak that punishes illness; minimum maintenance quests for health so one domain does not eat the others.
- Seasons of about 90 days with a theme and a boss objective chosen at the weekly review.
- Everything optional and removable; no dark patterns (no loss-aversion pop-ups, no guilt messages).
**Exit:** gaming the system (typing E0 claims) measurably gains less than doing the work; the owner can switch the whole layer off.

### L8 Collaboration and permissioned automation (L)
**Deliverables**
- The automation ladder in the settings and in each proposal. The system proposes automations it can do: create the daily note, import a statement, move files, open the right workspace, set a reminder, prepare a draft. Each proposal says what it will touch, the scope and the undo.
- Approvals are recorded (class, scope, expiry) and revocable in one click; every action is logged in the existing record and visible in the Flight Player.
- All actions go through Studio's existing safety gate; money, messages to other people and deletions stay at "ask every time".
- It also proposes things it should **not** automate, because doing them yourself is the point (practice, hard decisions).
**Exit:** three automation classes run at the "act within approved scope" level for 14 days without a mistake you report; every action has an undo or a confirmation; revoking works immediately.

### L9 Self-improvement of the harness (M)
**Deliverables**
- Signals: your ratings of advice, forecast accuracy (what the consequence maps predicted against what happened), loop adherence, how often advice was ignored and why.
- The system proposes changes to its own prompts, notes and thresholds as **approved diffs** (the same mechanism as Studio's R6c and R7), each proven by an A/B on the advisor test suite and the Studio evaluation suite; it never changes safety or consent rules by itself.
- A monthly "how the system did" report you can read in five minutes.
**Exit:** one approved improvement with a before and after on the suite; forecast accuracy is tracked.

### L10 Release readiness (S to M)
Privacy review of every data class, backup and restore, a crisis-protocol review, documentation for a second user, an Obsidian and LLM-wiki export format you can keep if you leave the system, and a decision on whether it is for you alone or for others (**NOT CERTAIN**; changes the consent and support work a lot).

## 5. Order, dependencies and what you can use when

| Order | Milestone | Needs (platform) | You can use |
| --- | --- | --- | --- |
| 1 | L0 | R8 slice, INF-15 or a local-only flag | Nothing visible; the safe base |
| 2 | L1 | Studio window pattern (from R6a/R6b), Gemma 4 | Your strategy, goals, assumptions, first plan |
| 3 | L2 | none | A dashboard of objectives and why each is at risk |
| 4 | L3 | R5 shell for scheduled prompts | The daily loop |
| 5 | L4 | K1 for kit tools (Studio-native readers until then) | Evidence-checked progress |
| 6 | L5 | R6c machinery, INF-6 structured output | The advisor you described |
| 7 | L6 | L4 and L5 | Honest course corrections |
| 8 | L7 | L2 and L4 | The Solo Leveling layer |
| 9 | L8 | gate (exists), K1 | Automation with your permission |
| 10 | L9 | R7 A/B | A system that improves itself under your control |
| 11 | L10 | all | A releasable version |

The first useful release is L0 to L3 (about 4 milestones). L4 is where the harness stops being a journal and becomes evidence-based, so do not skip it for L7.

## 6. Start collecting evidence now, with your current tools (no Rune needed)

So that day one of L1 has real data and so you can see your own calibration early, keep this one daily note in Obsidian, in this order. Five minutes at night.

```
date:
commitment (from this morning, one line):
done?: yes / partly / no
what exactly (one checkable detail, or "none"):
evidence I can point to (file, commit, number, link, or "none"):
energy 1-5 / sleep hours / movement (yes/no, what):
one thing I assumed today that might be wrong:
```

Do not score yourself on "how well". The fields "evidence I can point to" and "one assumption" are the seeds of L4 and the assumption register.

## 7. Impact on the other projects (to add to their handoffs if you agree)

- **rune-infer:** INF-15 (new): each model reports whether it is **local or cloud** in its model list, so Studio can refuse to send sensitive data to a cloud model. INF-6 (structured output) matters more: the advisor needs reliable JSON. Longer context handling for rolling summaries is **NOT CERTAIN**; Studio will measure first.
- **rune-kit and rune-tools:** nothing new is required for L0 to L3. For L4: filesystem read, git history and time exist as plugins (as read earlier). A **calendar** plugin and a **tabular/CSV read** helper are not known to exist (Studio can parse CSV itself, so this is optional). Whether kit will ever offer a scheduler is irrelevant: Studio schedules.
- **A2UI (K2):** finance import forms and progress widgets could be tool-provided later; the life window uses Studio's own components first.

## 8. Decisions needed from you

1. **Where the life module lives.** Default: inside Rune Studio (it has the window system, the record, the safety gate and the design language), with a clean boundary so it can be split out later. Alternative: a new project.
2. **Source of truth.** Default: a database (evidence, versions, audit) with Obsidian as a generated, editable view and an input. Alternative: the vault as truth (simpler, but evidence and versions get weak).
3. **Domains first.** Default: goals, skills, fitness and productivity first; finance read-only from imported files; mental health light-touch (reflection and habits, no diagnosis, no therapy claims).
4. **Passive activity sampling.** Default: off. Say if you want program-name-and-duration sampling in L4.
5. **Local-only for sensitive data.** Default: enforced and not overridable for finance, health and mind.
6. **Theme.** Default: Solo Leveling language as a skin only ("Hunter", "System", ranks), switchable, no outside assets.
7. **Who it is for.** Default: you alone through L9; L10 decides about others.

## 9. Risks, honestly

- **The model may not be good enough to challenge you, and models differ.** Because models are interchangeable, the advisor test suite (L5) runs per model, and the model for the `advisor` role is chosen from its results, not from its size or its reputation. Sensitive data only ever goes to a local model (INF-15, L0).
The model may not be good enough to challenge you. A small local model tends to agree. L1's spike and L5's suite exist to find out early; if Gemma 4 fails them, the options are a larger local model or a cloud model for non-sensitive reasoning only (a decision, not a default).
- **Goodhart's law.** Any number you can raise without improving yourself will get raised. The evidence grades and L7's order (game layer last) are the defence; they reduce it, they do not remove it.
- **Surveillance by accident.** Collecting evidence is collecting data about you. Per-source consent, no screenshots, local-only and delete are not extras.
- **False confidence.** Consequence maps and trajectory calls are estimates from an imperfect model; they are always labelled and later compared with reality (L9).
- **Dependence.** A system that decides for you makes you weaker. The automation ladder and the "do not automate" proposals are there on purpose.
- **Scope.** This is a large track on top of an unfinished platform track. If time is short, L0 to L3 plus section 6 is a complete, useful stopping point.
- **Not a clinician.** It is not a substitute for medical or mental-health care, and it says so where it matters.

## 10. NOT CERTAIN list

Encryption mechanism; each candidate model's ability to challenge rather than flatter; reliability of crisis-phrase handling by a small model; whether a calendar source exists; long-context behaviour for summaries; whether the vault round-trip (export then re-import edits) is reliable; how much daily time the loop really costs (measured in L3); whether the life module should ever serve other users (L10). None of these is assumed in the milestones above; each has the milestone that measures it.
