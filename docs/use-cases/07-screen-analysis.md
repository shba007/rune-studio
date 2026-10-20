# Ask about your screen

**Status:** Partly works. Pointing at things is unreliable, so it is off by default.

## What it is
Rune looks at your screen once and answers a question about it, with a list of findings and the evidence for each.

## How to use
1. Make sure no task is running, then click the orb to open the pill.
2. Click the analysis button on the pill. A separate window opens.
3. Type a question, for example `Summarise what is on the screen and point out anything unusual`, and send it.
4. Read the findings and their evidence. To cancel, close the window or press Ctrl+Alt+Esc.
5. Optional: in Settings turn on "analysisBoxes" to draw boxes on the picture. They are often wrong (see Limits).
6. To read a saved result later, run `rune-studio-cli analyses`, then `rune-studio-cli analysis <id>`.

## What happens
1. Rune takes one screenshot.
2. One model call returns up to 8 findings, each with a label, a description and evidence.
3. The analysis window shows them. This window is hidden from screenshots.
4. The result is saved in the record.

## What you will see
The window opens in about half a second, and the answer arrives after 15 to 49 seconds.

## How well it works
Measured: 14.5 s for 1 finding and 44 to 49 s for 8 (the limit is 60 s). A file with a fake key and password did not leak either value into the answer. It refuses to start while a task is running. Closing the window or pressing Stop cancels it cleanly.
Problem found: only 1 of 8 boxes was in the right place. Before the F1 fixes the whole feature failed every time (fixed since).

## Limits
- Current screen only. No files, no drag and drop, no voice or outside-agent trigger.
- After a cancel the server may keep working for a while (a rune-infer issue).
- Box accuracy was never measured with the probe script.

## Where it can grow
Better boxes by combining the server's grounding and text-reading tools, comparing two screens, files as input.

## Features used
One screenshot, a forced report tool, a separate window, the saved record.
