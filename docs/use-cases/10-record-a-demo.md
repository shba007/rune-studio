# Record a demo

**Status:** Planned (milestone R5). Nothing here is built yet.

## What it is

Make a screen video of Rune working, using an ordinary recorder such as OBS or Windows Game Bar.

## How to use (when built)

1. Open Rune Studio's Settings and switch on **Show Rune in screen recordings**.
2. Start OBS with **Display Capture** (or start Game Bar).
3. Run a task in Rune.
4. Stop the recording.
5. Switch the setting off again when you are done.

## The problem today

Rune hides its pill and its screen overlay (frame, bar and analysis boxes) from all screen capture, so the model never reads its own interface. That also hides them from Snipping Tool, Game Bar and OBS.

## What happens (planned)

1. The setting switches off the hiding for the three Rune windows, so recorders see them.
2. Rune still keeps its windows out of the screenshots the model gets, by hiding or masking them. The method is chosen by measuring.
3. An evaluation run with the setting off and on must show no loss in success or speed.
4. A built-in recorder is not planned. It would need a video encoder, audio and storage, and the same hiding problem would remain.

## What you will see (planned)

The pill, frame, captions and ripples in your video.

## How well it works

Not measured. It is not built.

## Limits (expected)

- OBS window capture does not see the click-through frame, so use display capture.
- Exclusive-fullscreen apps may not record.

## Where it can grow

A recording-ready mode with a larger pill and a timer.

## Features used (planned)

A new setting, the capture-hiding switch for Rune's windows, the screenshot masking.

Specification: the "Agent identity, demo mode and the idle review" section of `docs/ARCHITECTURE.md`.