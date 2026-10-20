# 11 Idle self-review

**Status:** Planned (milestones R5 and R6).

## What it is
When you have not used Rune for a while, it offers to review its recent tasks and write notes about what went wrong.

## The plan
1. After 10 idle minutes the pill asks: "Rune wants to review its recent tasks." with **Start** and **Deny**. With no answer in 30 seconds it starts. Deny skips this round.
2. A calm "Reflecting" orb and a progress ring show it working.
3. Any new task, the hotkey, Stop or an outside agent cancels it within a second.
4. R5 ships only an empty shell. R6 adds the real work: for failed or slow runs it finds the likely cause and proposes a **pending note** that you accept or reject.
5. R7 puts accepted notes into the prompt and keeps them only if an A/B test shows no loss.

## Rules
- Notes and reports only. Prompts and safety limits change only through a change you approve.
- Off by default until R6.

## Expected limits
- rune-infer has no request priority, so a review already running can delay your next task until it ends.
- Whether the notes actually help is **not known** until R7 measures it.

See the roadmap: `docs/ROADMAP.md` (R5 to R7).
