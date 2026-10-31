# Talk to it

**Status:** Partly works. The model often mishears.

## What it is

You speak a task instead of typing it. Rune listens hands-free once you click the microphone.

## How to use

1. Connect a microphone, click the orb to open the pill, then click the microphone once.
2. Say the task, for example "Open Notepad and write hello".
3. Read your words on the pill. The task starts after 2 seconds. To cancel it in that time, press Ctrl+Alt+Esc.
4. While a task runs you can say a control word: "stop", "yes" or "no". These act at once.

## What happens

1. A detector cuts your speech into separate utterances.
2. The same model that reads the screen hears the audio (there is no separate speech-to-text model) and writes down the words.
3. A control word acts immediately. A task is shown on the pill and then starts like a typed task.

## What you will see

The orb follows your voice level, then your words appear on the pill.

## How well it works

Measured: the audio is accepted and heard, but understanding is poor. Out of 5 recordings a speech model got all 5 right, while this model returned "Open PowerPoint", "Open terminal" and nonsense. "Open Notepad and write hello" once started a Calculator task. Since then the F1 fixes added a "transcribe only" instruction, a length cap, rejection of repeated words, logging of every ignored result and the 2-second window (none seen working yet).

## Limits

- Only tested with speakers into the microphone. "Stop", "yes" and "no" with a real voice have not been tested.
- There is no Cancel button. Use the Stop hotkey or the tray menu.

## Where it can grow

A real speech-to-text endpoint on rune-infer (the biggest gain), a Cancel button, a wake word.

## Features used

Voice detector, audio sent to the model, the 2-second delay, control words.