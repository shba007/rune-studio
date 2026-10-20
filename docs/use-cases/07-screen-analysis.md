# Ask about your screen

**Status:** Built, not seen on a real screen. The boxes it draws on your screen are always on, and they are often wrong until the probe is run (see Limits).

## What it is

Rune looks at your screen once and answers a question about it, with a list of findings and the evidence for each.

## How to use

1. Make sure no task is running, then click the orb to open the pill.
2. Click the scan button on the pill. A bar appears at the top of the screen and cards grow upward from the orb.
3. Optionally type a question in the first card, for example `Summarise what is on the screen and point out anything unusual`, and press Enter. An empty question means "describe it".
4. Read the answer, then open the findings one by one: each shows its description and the evidence, and its box on the screen goes to full strength. To cancel, press the red Cancel button on the card or Ctrl+Alt+Esc.
5. Escape collapses the open finding, then clears the selection, then closes the cards (which also clears the boxes).
6. To read a saved result later, run `rune-studio-cli analyses`, then `rune-studio-cli analysis <id>`.

## What happens

1. Rune takes one screenshot, and a scan line sweeps the screen once.
2. One model call returns up to 8 findings, each with a label, a description, evidence and, if it could, a box.
3. A guard drops boxes that are plainly unusable (tiny, huge, off-screen, duplicates, or a set that tiles the whole screen). Findings stay.
4. Dotted boxes with numbered badges are drawn on your real screen. If the screen changes (another window, or more than 5% of it moves) the boxes fade out for good and the cards say SCREEN CHANGED.
5. The result is saved in the record.

## What you will see

A black bar at the top (the same style as the control bar) with the stage and a timer, a dot skeleton while it works, then the answer (four lines, MORE for the rest) and a list of findings. The answer arrives after 15 to 49 seconds. Everything Rune draws is hidden from screenshots.

## How well it works

Measured: 14.5 s for 1 finding and 44 to 49 s for 8 (the limit is 60 s). A file with a fake key and password did not leak either value into the answer. It refuses to start while a task is running. Closing the window or pressing Stop cancels it cleanly.
Problem found: only 1 of 8 boxes was in the right place. Before the F1 fixes the whole feature failed every time (fixed since).

## Limits

- Current screen only. No files, no drag and drop, no voice or outside-agent trigger.
- After a cancel the server may keep working for a while (a rune-infer issue).
- **The boxes cannot be turned off, and their accuracy is unmeasured.** In the field test 1 of 8 was right. The guard removes boxes that are plainly implausible but cannot know a plausible one is wrong. Run `pwsh scripts/m15/run-probe.ps1 -Modes "gemma-4-12b-it:analysis,gemma-4-12b-it:analysis-point"` to measure it.
- Boxes are drawn on the primary monitor only.

## Where it can grow

Better boxes by combining the server's grounding and text-reading tools, comparing two screens, files as input.

## Features used

One screenshot, a forced report tool, the screen overlay and the pill's cards, the saved record.