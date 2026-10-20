# 07 Ask about your screen

**Status:** Partly works. Pointing at things is unreliable, so it is off by default.

## What it is
Rune looks at your screen once and answers a question about it, with a list of findings and the evidence for each.

## Try it
Press the analysis button on the pill and ask: **Summarise what is on the screen and point out anything unusual.**

## What happens
1. Rune takes one picture of the screen.
2. One model call returns up to 8 findings, each with a label, a description and evidence.
3. A separate analysis window shows them. The window is hidden from screenshots.
4. The result is saved (`rune-studio-cli analyses`).

## What you will see
A window that opens in about half a second, then the answer after 15 to 49 seconds.

## How well it works
Measured: 14.5 s for 1 finding, 44 to 49 s for 8 (limit 60 s). A file with a fake key and password did not leak either value into the answer. It refuses to start while a task is running; Stop or closing the window cancels it.
**Problem found:** only 1 of 8 boxes was in the right place, so boxes are off by default (a setting turns them on). Before F1 the feature failed every time (fixed).

## Limits
- Current screen only. No files, no drag and drop, no voice or outside-agent trigger.
- After a cancel the server may keep working for a while (a rune-infer issue).
- Box accuracy was never measured with the probe.

## Where it can grow
Better boxes by combining the server's grounding and text-reading tools, comparing two screens, files as input.

## Features used
One screenshot, a forced report tool, a separate window, the saved record.
