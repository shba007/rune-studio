# Open an app and type

**Status:** Works.

## What it is
You give Rune a task in words. Rune opens the app, clicks where needed and types the text for you.

## How to use
1. Click the orb to open the pill.
2. Type a task in the box, for example `Open Notepad and write hello`.
3. Press Enter, and take your hands off the mouse and keyboard.
4. To stop at any moment, press Ctrl+Alt+Esc (the default Stop hotkey).

## What happens
Rune repeats a loop until the task is done:
1. It makes a short plan of one to three goals.
2. It takes a screenshot and reads the list of controls on the screen.
3. It picks one action, for example "open Notepad".
4. The safety gate checks the action. Low-risk actions go straight through.
5. The action runs, then Rune waits about 2 seconds for the screen to settle.
6. It goes back to step 2, for example to "type hello", until it reports done.

## What you will see
A frame around the screen, a short caption for each action, a ripple where it clicks, and the plan shown as arcs around the orb.

## How well it works
Measured on Windows 11: 55 seconds and 6 steps, and Notepad contained "hello". Two of the six steps were wasted because the stuck detector fired wrongly (fixed since, not yet re-measured). Stop took 420 ms. Per step: looking at the screen 0.3 s, acting 0.04 s, settling 1.8 s, the model about 4.5 s.

## Limits
- With thinking switched on it is about 4 times slower (runs of 93 to 169 s), so thinking is off by default.
- Rune does one action per model call, so long tasks take minutes.
- Some Store apps cannot be opened by name yet.
- Not tested: apps run as administrator, display scaling of 125% or 150%, a second monitor.

## Where it can grow
Faster steps, remembered tips per app (for example "Notepad restores old tabs"), more than one monitor.

## Features used
Text input, planning, screenshot and control list, the action loop, the safety gate, stuck detection, the flight recorder.
