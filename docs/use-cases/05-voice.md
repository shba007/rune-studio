# 05 Talk to it

**Status:** Partly works. The model often mishears.

## What it is
Click the microphone once, then speak. Rune listens hands-free.

## Try it
Say: **Open Notepad and write hello**

## What happens
1. A detector cuts your speech into utterances.
2. The same model that looks at the screen hears the audio (there is no separate speech-to-text model).
3. Short control words (stop, yes, no) act immediately.
4. A spoken task is shown on the pill and starts after **2 seconds**. Press Stop in that time to cancel.

## What you will see
The orb follows your voice level, then your words appear on the pill.

## How well it works
Measured: the audio is accepted and heard. Understanding is poor: of 5 recordings a speech model got 5 right, this model returned "Open PowerPoint", "Open terminal" and nonsense, and "Open Notepad and write hello" once started a Calculator task. F1 added a "transcribe only" instruction, a length cap, rejection of repeated words, logging of every ignored result and the 2-second window (all not yet seen working).

## Limits
- Tested only with speakers into the microphone. Stop, yes and no with a real voice: **not tested**.
- There is no Cancel button; use the Stop hotkey or the tray.

## Where it can grow
A real speech-to-text endpoint on rune-infer (the biggest gain), a Cancel button, a wake word.

## Features used
Voice detector, audio to the model, 2 s delay, control words.
