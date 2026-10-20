# Risky actions need your Yes

**Status:** Partly works. The main fix has not yet been seen working on Windows.

## What it is
Before a risky action (deleting, paying, sending) Rune stops and asks you to approve it.

## How to use
1. Optional: in Settings turn on "Ask before every action" if you want a question for nearly every action.
2. Click the orb to open the pill.
3. Type a task that includes a risky step, for example `Delete the file report-old.txt on the Desktop`.
4. Press Enter. When the card appears on the pill, click **Yes** to allow the action or **No** to refuse it. You can also say "yes" or "no".
5. To stop the whole task, press Ctrl+Alt+Esc.

## What happens
1. The model proposes an action and says how risky it thinks it is.
2. The safety gate decides from fixed rules, the model's own label and, if enabled, a second opinion from a small model.
3. If the action is risky, the card asks "Allow?".
4. Yes: Rune returns focus to the window that was in front, then runs the action. No, or no answer in 120 seconds: the action is refused and the model is told.

## What you will see
A card with the question and a countdown, then either the action or a refusal.

## How well it works
Measured: the card appears, No is respected, and strict mode asks for key presses, clicks and opening apps but not for scrolling or waiting.
A bug was found: after Yes, Rune itself became the front window, so the approved action hit Rune and not your file. The F1 fix returns focus and refuses the action if it cannot. This has not been seen working on Windows and should be the first thing re-tested.

## Limits
- The second-opinion check uses the probability of the chosen answer. "Click Pay now" scored 0.587, below the 0.8 limit, so that check alone would not ask.
- Not tested: three No answers in a row, the 2-minute timeout, answering by voice. A spoken "yes" never approves an approval-type prompt, by design.
- Text you type is stored in the record, including passwords.

## Where it can grow
Risk rules per app, hiding secrets in the record, a dry-run mode that only proposes actions.

## Features used
Safety gate, confirm card, strict mode, spoken yes and no.
