# Browse the web

**Status:** Partly works.

## What it is
Rune opens a web page in your default browser and works on it: closes a popup, clicks a small button, scrolls.

## How to use
1. Start Rune Studio and make sure the model server (rune-infer) is running.
2. Click the orb to open the pill.
3. Type a task that names the page, for example `Open example.com and close the cookie banner`.
4. Press Enter, and take your hands off the mouse and keyboard.
5. To stop at any moment, press Ctrl+Alt+Esc.
6. Optional: in Settings you can switch on "fast checks". Leave them off on a server that holds only one model (see Limits).

## What happens
1. Rune opens the address in the browser.
2. It takes a screenshot. Web pages show few controls to Windows, so Rune mostly clicks by position on a grid from 0 to 1000. It can ask for a zoomed picture of one region to aim better.
3. It picks one action, the safety gate checks it, and it runs.
4. Rune repeats from step 2 until the task is done or it is stuck.

## What you will see
The page opens, with a ripple for each click and a caption for each action.

## How well it works
Measured: the "close a popup" and "click a small target" test tasks passed. In one partial test run only 4 of 13 real tasks passed, but two bugs distorted that run, so it is not the true pass rate. In an earlier probe, small buttons were hit 57 to 79 % of the time.

## Limits
- Small targets are the weak spot.
- Web addresses with a `?query` lost the query before the F1 fixes (fixed since, not yet seen working).
- Fast checks use a small second model. On a server that holds one model at a time, each check swaps models and costs 7 to 16 seconds, and its answers on real screens were unreliable. They are off by default.

## Where it can grow
Control pages through the page structure (rune-kit's browser tool) instead of pixels, and a server that keeps both models loaded.

## Features used
App opening with an address, click, scroll, zoomed screenshot, stuck detection, optional fast checks.
