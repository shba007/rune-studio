# Evaluation baselines

A baseline is one full run of the suite, kept in the repository, so that any later change can be
judged against it with `scripts/eval/report.mjs --compare`. The rule: **success rate not lower than the
baseline's by more than one run, and median task duration not more than 10% higher.**

## Baseline storage convention: `baselines/<model>/`

Baselines are stored per model under `docs/evals/baselines/<model>/`:

```
docs/evals/baselines/<model>/baseline-<YYYY-MM-DD>-fast-off.md             (report.md)
docs/evals/baselines/<model>/baseline-<YYYY-MM-DD>-fast-off.results.jsonl  (results.jsonl)
docs/evals/baselines/<model>/baseline-<YYYY-MM-DD>-fast-off.env.json       (env.json)
```

and the same for `-fast-on`.

Every `results.jsonl` row and `env.json` file records the served `model` and the resolved `profile` (including coordinate pointing convention, thinking switch style, image limits, and tool requirements).

- **Same-model comparisons**: When `report.mjs --compare` compares two runs sharing the same model and profile, it enforces the gate rule: success rate not lower by more than one run, and median task duration within +10%. Exit code is 1 on `FAIL`.
- **Cross-model comparisons**: When comparing runs with different models or profiles, the harness outputs a **model comparison table** instead (success rate, median duration, steps, tokens, and boxes quality if recorded). It never emits a `PASS` or `FAIL` verdict and exits with code 0.
- **Maintainer only**: Baseline data is not committed by agents without live model access; the maintainer produces and commits baseline sets after measuring real model performance.

## Make a baseline (the maintainer does this, on Windows)

1. Windows 11, a clean desktop (no Notepad, Calculator, Settings or Paint open), rune-infer running,
   Rune Studio closed. Read `scripts/eval/README.md`: the run moves your real mouse and types.
2. Check the plan: `pwsh scripts/eval/run.ps1 -DryRun`
3. Run it twice, with the fast checks off and on:

   ```powershell
   pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks off -Out eval-out\baseline-off
   pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks on  -Out eval-out\baseline-on
   ```

   Each prints where `results.jsonl`, `env.json` and `report.md` are. If it refuses to start, read
   `preflight.txt` in the output folder.

4. Look at the report first. Runs with an `error` column are setup, teardown or read-back problems, not
   agent failures: fix the cause and run again rather than committing a broken baseline.
5. Store in `docs/evals/baselines/<model>/`, for each run, three files named by date and setting:

   ```
   docs/evals/baselines/<model>/baseline-<YYYY-MM-DD>-fast-off.md             (report.md)
   docs/evals/baselines/<model>/baseline-<YYYY-MM-DD>-fast-off.results.jsonl  (results.jsonl)
   docs/evals/baselines/<model>/baseline-<YYYY-MM-DD>-fast-off.env.json       (env.json)
   ```

   and the same with `fast-on`. `env.json` records the model, resolved profile, the settings snapshot (no key and no URL),
   the app version and git commit, and a hash of the suite, so a later run can be checked against the
   same suite and model.

Do not commit the output folder itself: `work\` holds scratch files and `suite.json` is a copy.

## Judge a change against a baseline

```powershell
pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks off -Out eval-out\candidate
node scripts/eval/report.mjs eval-out\candidate\results.jsonl --compare docs\evals\baseline-<date>-fast-off.results.jsonl
```

The first line of the report is `Gate: PASS` or `Gate: FAIL` with the reason; the exit code is 1 on FAIL.
With three repeats of 15 tasks (45 runs) one run is about 2 percentage points, so the gate is
indicative, not a statistical test: look at the per-task table and at where the model and the check
disagree before trusting a verdict either way.

## Measure the verifier (R3)

The independent check of a finished task (`verifyMode`, see ARCHITECTURE.md "The verifier") is judged
from three runs of the suite: the check off, in the background, and blocking. In eval runs the check is
**off unless the suite says otherwise**, so the baselines above stay comparable. Make a suite file per
mode by setting `config.verify_mode` (`"off"`, `"background"` or `"blocking"`), for example copies of
`scripts/eval/suite.desktop-15.json` named `suite.verify-background.json` and `suite.verify-blocking.json`.
Fast checks stay as in your baseline (use the same `-FastChecks` for all three).

```powershell
pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks off -Out eval-out\verify-off
pwsh scripts/eval/run.ps1 -Suite scripts\eval\suite.verify-background.json -Repeats 3 -FastChecks off -Out eval-out\verify-background
pwsh scripts/eval/run.ps1 -Suite scripts\eval\suite.verify-blocking.json   -Repeats 3 -FastChecks off -Out eval-out\verify-blocking
```

Then, for each of the two checking runs, compare with the one that had the check off and read the report:

```powershell
node scripts/eval/report.mjs eval-out\verify-background\results.jsonl --compare eval-out\verify-off\results.jsonl --out eval-out\verify-background\report.md
node scripts/eval/report.mjs eval-out\verify-blocking\results.jsonl   --compare eval-out\verify-off\results.jsonl --out eval-out\verify-blocking\report.md
```

What to look at:

- **Gate and overhead** (top of the report): success not lower, median task duration within +10%. A
  background check should add nothing to the duration (it runs after the task, and the harness waits
  for it outside the task's clock); a blocking check adds one model call per claimed success and is
  expected to be near or over the limit.
- **Verifier vs the checkers**: the confusion matrix of the verdict against the machine's check, the
  agreement among decisive verdicts, **false contradictions** (the check said the task was not done but
  the machine says it was) and **missed failures** (the check said done but the machine says not). The
  background and blocking runs make the same call on the same kind of screen; the numbers should be close.
- Rows with verdict `timeout` or `error` are checks that did not finish; many of them mean the model
  server is too slow or busy for the check, not that the check is wrong.

Commit the three reports with their `results.jsonl` and `env.json`, named `verify-<mode>-<YYYY-MM-DD>`.
Until they exist, R3 is built, not measured.