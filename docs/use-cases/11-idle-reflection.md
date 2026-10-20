# Idle self-review

**Status:** Planned (milestones R5 and R6). Nothing here is built yet.

## What it is
When you have not used Rune for a while, it offers to review its recent tasks and write notes about what went wrong.

## How to use (when built)
1. Open Rune Studio's Settings and switch on reflection. It is off by default.
2. Optional: set the idle time (default 10 minutes).
3. Leave the PC alone. When the time passes, the pill asks "Rune wants to review its recent tasks" with **Start** and **Deny**.
4. Click **Start**, click **Deny** to skip this round, or do nothing: with no answer in 30 seconds it starts.
5. Later, read the proposed notes and accept or reject each one.
6. To interrupt a review, start a task, press Ctrl+Alt+Esc or use the hotkey. It stops within a second.

## What happens (planned)
1. The pill shows the question. A calm "Reflecting" orb and a progress ring show the work.
2. In R5 the review is an empty shell. In R6 it studies failed or slow runs, finds the likely cause and proposes a pending note.
3. In R7, accepted notes are added to the prompt, and kept only if an A/B test shows no loss.

## What you will see (planned)
The question card, the calm orb, a progress ring, then a list of pending notes.

## How well it works
Not measured. Whether the notes actually help is unknown until R7 measures it.

## Limits (expected)
- rune-infer has no request priority, so a review already running can delay your next task until it ends.
- Notes and reports only. Prompts and safety limits change only through a change you approve.

## Where it can grow
Skills built from repeated successful runs, safety limits tuned from your Yes and No answers, choosing a model per task.

## Features used (planned)
The idle timer, the Reflecting state, the record of past runs, notes storage.

See the roadmap: `docs/ROADMAP.md` (R5 to R7).
