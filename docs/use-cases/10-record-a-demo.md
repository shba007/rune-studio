# 10 Record a demo

**Status:** Planned (milestone R5).

## What it is
Make a screen video of Rune working, with an ordinary recorder.

## The problem today
Rune hides its pill, its screen frame and its analysis window from all screen capture, so the model never reads its own interface. That also hides them from Snipping Tool, Game Bar and OBS.

## The plan
- Use an **external recorder** (OBS or Game Bar). A built-in recorder is not planned: it would need a video encoder, audio and storage, and the same hiding problem would remain.
- A new setting, **Show Rune in screen recordings** (off by default), lets recorders see Rune's windows.
- With it on, Rune still keeps its windows out of what the model sees, by hiding or masking them. The method is chosen by measuring.
- A before and after evaluation run must show no loss in success or speed.

## Expected limits (not tested)
- OBS window capture does not see the click-through frame; use display capture.
- Exclusive-fullscreen apps may not record.

See the milestone: `docs/handoff/R5-reflection-shell-agent-identity-demo.md` (part R5b).
