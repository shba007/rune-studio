# 06 Let another AI drive

**Status:** Works. Agent colours and icons are planned (R5).

## What it is
An AI tool such as Claude Code or Antigravity uses your PC through Rune, with the same screen frame and the same safety gate.

## Try it
1. In a terminal: `rune-studio-cli connect claude` (or `antigravity`), then restart that tool.
2. Turn on **Allow outside agent** in Settings (or run `rune-studio-cli enable`).
3. Ask the tool: "Use Rune Studio to open Notepad and type hello."

## What happens
1. The agent starts with `rune_begin`. A frame appears and the top bar says who is in control.
2. It sends a plan, then repeats: look (`rune_observe`), act (click, type, keys, open app, scroll, drag, wait). Each action returns a fresh screenshot.
3. Risky actions wait for your Yes, exactly as for the built-in agent.
4. It ends with `rune_done`. Stop at any moment ends its control; later calls answer "stopped by user" until you re-enable.

## What you will see
The frame, a bar reading "<agent> is in control" with the current action, ripples on clicks, a red Stop button.

## How well it works
Measured: begin, actions, Stop, re-enable and overlapping looks all behaved. (Connecting Claude was not run, because it edits your Claude settings.)

## Limits
- The agent's name is what it says it is. It is not proven, and every agent gets the same safety checks.
- Only one agent at a time; a silent agent loses control after a while.
- The quality check after a task does not run for outside agents.

## Where it can grow (R5)
Each agent gets its own accent colour and icon in the orb and the top bar, with the connection recording which agent it is. Later: a quality check for outside runs and per-agent permissions.

## Features used
The MCP bridge on 127.0.0.1:47600, the access token, the safety gate, the lease, the overlay.
