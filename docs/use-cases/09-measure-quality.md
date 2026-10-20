# 09 Measure its quality

**Status:** Partly works. There is no trustworthy baseline yet.

## What it is
Two tools: an **evaluation suite** that runs 15 fixed tasks and checks the real result, and a **verifier** that looks at the final screen after a task and says whether the claimed success is true.

## Try it
- Suite: `scripts/eval/run.ps1` (needs a build with the `eval` feature and a model server).
- Verifier: Settings, `verifyMode`: off, background (default) or blocking.

## What happens
1. The suite sets up a fixture, runs the task, and a checker that does not trust the model decides pass or fail. `scripts/eval/report.mjs` compares runs and applies a gate: success not lower by more than one run, median time not more than 10% worse.
2. The verifier asks the model "does the final screen match the claim?" and records verified, contradicted or unsure. A contradiction shows a notice.

## How well it works
Measured: one partial baseline (6 of 15 passed) that is **not valid**, because two bugs distorted it. Verifier on Notepad: 7 verified, 2 false contradictions, 1 inconsistent (it called Notepad "VS Code"; F1 now gives it the real window title, effect not measured). Blocking mode took 3.5 s. Stop and a new task cancel a pending check.

## Limits
- The 45-run baseline, the three-mode verifier comparison and replay are **not run**.
- Everything F1 changed must be re-measured on a machine with the model.

## Where it can grow
A committed baseline per model, a release gate in CI, replay of stored runs.

## Features used
The eval feature, checkers, the report script, the verifier.
