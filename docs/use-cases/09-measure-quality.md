# Measure its quality

**Status:** Partly works. There is no trustworthy baseline yet.

## What it is
Two tools. An evaluation suite runs 15 fixed tasks and checks the real result. A verifier looks at the final screen after a task and says whether the claimed success is true.

## How to use
Evaluation suite:
1. Build Rune Studio with the `eval` feature (it is not in releases).
2. Make sure the model server is running and no Notepad or Calculator is open.
3. Run `scripts/eval/run.ps1`.
4. Compare two runs with `node scripts/eval/report.mjs`.

Verifier:
1. Open Rune Studio's Settings.
2. Set `verifyMode` to `off`, `background` (the default) or `blocking`.
3. Run a task. A notice appears only when the check contradicts the claimed success. Verdicts are in `rune-studio-cli run <id>`.

## What happens
1. The suite sets up a fixture, runs the task, and a checker that does not trust the model decides pass or fail.
2. The report applies a gate: success may not drop by more than one run, and median time may not be more than 10% worse.
3. The verifier asks the model "does the final screen match the claim?" and records verified, contradicted or unsure.

## What you will see
A results folder and a comparison table for the suite. A short notice for the verifier, only on a contradiction.

## How well it works
Measured: one partial baseline (6 of 15 passed) that is not valid, because two bugs distorted it. Verifier on Notepad: 7 verified, 2 false contradictions, 1 inconsistent (it called Notepad "VS Code"; the F1 fixes now give it the real window title, effect not measured). Blocking mode took 3.5 s. Stop and a new task cancel a pending check.

## Limits
- The 45-run baseline, the three-mode verifier comparison and replay have not been run.
- Everything the F1 fixes changed must be re-measured on a machine with the model.

## Where it can grow
A committed baseline per model, a release gate in CI, replay of stored runs.

## Features used
The eval feature, checkers, the report script, the verifier.
