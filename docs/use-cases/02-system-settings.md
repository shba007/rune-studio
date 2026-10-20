# System settings and calculator

**Status:** Works.

## What it is
Rune opens a built-in Windows app, such as Settings or Calculator, and operates it for you.

## How to use
1. Start Rune Studio and make sure the model server (rune-infer) is running.
2. Click the orb to open the pill.
3. Type one task in the box. Two examples: `Open Settings`, or `Open the calculator and compute 12 times 13`.
4. Press Enter, and take your hands off the mouse and keyboard.
5. To stop at any moment, press Ctrl+Alt+Esc.

## What happens
1. Rune makes a short plan.
2. It opens the app by name ("settings" is turned into the Windows Settings address, "calculator" into Calculator).
3. It takes a screenshot and reads the list of controls in the front window.
4. It clicks a control by its id from that list, which is exact, instead of guessing a position on the picture. It also uses key presses.
5. The safety gate checks each action, and Rune repeats from step 3 until the task is done.

## What you will see
The app opens and its buttons are pressed one by one, with a caption for each action and a frame around the screen.

## How well it works
Measured: both examples passed, and the end-of-task check said "verified" with correct evidence.

## Limits
- Before the F1 fixes, `Open Settings` and `Calculator` could not be opened by name (error 2). Fixed since, not yet seen working on Windows.
- A Windows admin-permission (UAC) prompt cannot be seen or answered by Rune.
- A locked or sleeping PC stops the task with "screen unavailable".

## Where it can grow
A list of known apps with their shortcuts, and direct tools (through rune-kit) that change a setting without clicking.

## Features used
App opening with aliases, control ids, key presses, the safety gate, the end-of-task check.
