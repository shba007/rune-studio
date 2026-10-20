# 04 Risky actions need your Yes

**Status:** Partly works. The key fix is not yet seen working on Windows.

## What it is
Before a risky action (deleting, paying, sending) Rune stops and asks you.

## Try it
**Delete the file report-old.txt on the Desktop**

## What happens
1. The model proposes an action and says how risky it thinks it is.
2. The safety gate decides from fixed rules, the model's own label and, if enabled, a second opinion.
3. If it is risky, a card on the pill asks **Allow? Yes / No**.
4. Yes: focus goes back to the window that was in front, then the action runs. No, or no answer in 120 seconds: the action is refused and the model is told.
5. In strict mode ("Ask before every action") nearly everything asks.

## What you will see
The card, with a countdown. Your answer, then either the action or a refusal.

## How well it works
Measured: the card appears, No is respected, strict mode asks for keys, clicks and opening apps but not for scrolling or waiting.
**Problem found:** after Yes, Rune itself became the front window, so the approved action hit Rune and not your file. F1 fixes this by returning focus and by refusing the action if it cannot. **Re-test this first on Windows.**

## Limits
- The second-opinion check uses the probability of the chosen answer. "Click Pay now" scored 0.587, below the 0.8 limit, so it still would not ask on that check alone.
- Not tested: three No answers in a row, the 2-minute timeout, answering by voice. A spoken yes never approves an "approval" prompt, by design.
- Text you type is stored in the record, including passwords.

## Where it can grow
Per-app risk rules, hiding secrets in the record, a dry-run mode that only proposes.

## Features used
Safety gate, confirm card, strict mode, voice yes and no.
