# 01 Open an app and type

**Status:** Works.

## What it is
You type or say a task. Rune opens the app, clicks where needed and types the text for you.

## Try it
Type in the pill: **Open Notepad and write hello**

## What happens
1. Rune makes a short plan (one to three goals). The goals appear as arcs around the orb.
2. It looks at the screen and the list of controls on it.
3. It picks one action, for example "open Notepad".
4. The safety gate checks the action. Low-risk actions go straight through.
5. The action runs. Rune waits about 2 seconds for the screen to settle.
6. Steps 2 to 5 repeat, for example "type hello", until Rune reports done.

## What you will see
A frame around the screen, a short caption for each action, a ripple where it clicks, and the plan arcs on the orb. Stop ends it at once.

## How well it works
Measured: 55 seconds and 6 steps, and Notepad contained "hello". Two of the six steps were wasted because the stuck detector fired wrongly (fixed in F1, not yet re-measured). Stop took 420 ms. Each step: looking at the screen 0.3 s, acting 0.04 s, settling 1.8 s, the model about 4.5 s.

## Limits
- With thinking switched on it is about 4 times slower (runs of 93 to 169 s), so thinking is off by default.
- One action per model call, so long tasks take minutes.
- Some Store apps cannot be opened by name yet.
- Windows run as administrator: **not tested**. Display scaling of 125% or 150% and a second monitor: **not tested**.

## Where it can grow
Faster steps, remembered tips per app ("Notepad restores old tabs"), more than one monitor.

## Features used
Text or voice input, planning, screen and control list, the action loop, the safety gate, stuck detection, the flight recorder, the verifier.
