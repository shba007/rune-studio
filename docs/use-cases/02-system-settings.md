# 02 System settings and calculator

**Status:** Works.

## What it is
Rune opens built-in Windows apps and operates them, such as Settings and Calculator.

## Try it
- **Open Settings**
- **Open the calculator and compute 12 times 13**

## What happens
The same loop as [01](01-open-app-and-type.md). For native apps Rune reads the list of controls on the screen and clicks a control by its id, which is exact, instead of guessing a position.

## What you will see
The app opens, buttons are pressed one by one, captions name each action.

## How well it works
Measured: both tasks passed, and the checker that looks at the final screen said "verified" with correct evidence.

## Limits
- Before F1, "settings" and "Calculator" could not be opened by name (error 2). Fixed in F1, not yet seen working on Windows.
- A UAC (admin permission) prompt cannot be seen or answered by Rune.
- A locked or sleeping PC stops the task ("screen unavailable").

## Where it can grow
A list of known apps with their shortcuts, and direct tools through rune-kit that change a setting without clicking.

## Features used
App opening with aliases, control ids, key presses, the verifier.
