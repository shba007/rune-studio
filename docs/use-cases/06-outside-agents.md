# Let another AI drive

**Status:** Works. Agent colours and icons are planned (milestone R5).

## What it is

An AI tool such as Claude Code or Antigravity uses your PC through Rune, with the same screen frame and the same safety gate as Rune's own agent.

## How to use

1. Connect the tool. In a terminal run `rune-studio-cli connect claude` (or `rune-studio-cli connect antigravity`), then restart that tool.
2. Allow outside agents: switch on **Allow outside agent** in Rune Studio's Settings, or run `rune-studio-cli enable`.
3. In the tool, ask for the task, for example `Use Rune Studio to open Notepad and type hello`.
4. Watch the screen. Approve or refuse any card that asks about a risky action.
5. To take control back, press Ctrl+Alt+Esc. The tool is locked out until you run `rune-studio-cli enable` again.

## What happens

1. The tool starts with `rune_begin`. A frame appears and the top bar says who is in control.
2. It sends a plan, then repeats: look (`rune_observe`) and act (click, type, keys, open an app, scroll, drag, wait). Each action returns a fresh screenshot.
3. Risky actions wait for your Yes.
4. It ends with `rune_done` and a summary.

## What you will see

The frame, a bar reading "<agent> is in control" with the current action, ripples on clicks and a red Stop button.

## How well it works

Measured: begin, actions, Stop, re-enabling and overlapping looks all behaved. Connecting Claude was not run, because it edits your Claude settings.

## Limits

- The agent's name is whatever it says it is. It is not proven, and every agent gets the same safety checks.
- Only one agent works at a time. A silent agent loses control after a while.
- The end-of-task quality check does not run for outside agents.

## Where it can grow

Milestone R5 gives each known agent its own accent colour and icon in the orb and the top bar. Later: a quality check for outside runs and permissions per agent.

## Features used

The agent bridge at 127.0.0.1:47600, the access token, the safety gate, the one-at-a-time lease, the screen frame.