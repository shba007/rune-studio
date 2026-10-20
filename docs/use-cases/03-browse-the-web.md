# 03 Browse the web

**Status:** Partly works.

## What it is
Rune opens a web page and works on it: closes a popup, clicks a small button, scrolls.

## Try it
**Open example.com and close the cookie banner**

## What happens
Same loop as [01](01-open-app-and-type.md). Web pages show few controls to Windows, so Rune usually clicks by position on a 0 to 1000 grid. It can ask for a zoomed picture of one region to aim better.

## What you will see
The page opens in your default browser, a ripple shows each click.

## How well it works
Measured: the "dismiss popup" and "small target" test tasks passed. In the one partial test run, 4 of 13 real tasks passed, but that run was distorted by two bugs, so it is **not** the true pass rate. Earlier probing: small buttons were hit 57 to 79 % of the time.

## Limits
- Web addresses with a `?query` lost the query before F1 (fixed, not yet seen working).
- "Fast checks" (a small second model that answers yes or no about the screen) are **off by default**: on a server holding one model at a time each check swaps models and costs 7 to 16 seconds. Its answers on real screens were also unreliable.
- Small targets are the weak spot.

## Where it can grow
Control pages through the DOM (rune-kit's browser tool) instead of pixels, and a server that keeps both models loaded.

## Features used
App opening with a URL, click, scroll, zoomed look, stuck detection, optional fast checks.
