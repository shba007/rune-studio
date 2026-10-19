# Evaluation baselines

A baseline is one full run of the suite, kept in the repository, so that any later change can be
judged against it with `scripts/eval/report.mjs --compare`. The rule: **success rate not lower than the
baseline's by more than one run, and median task duration not more than 10% higher.**

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
5. Commit, for each run, three files named by date and setting:

   ```
   docs/evals/baseline-<YYYY-MM-DD>-fast-off.md             (report.md)
   docs/evals/baseline-<YYYY-MM-DD>-fast-off.results.jsonl  (results.jsonl)
   docs/evals/baseline-<YYYY-MM-DD>-fast-off.env.json       (env.json)
   ```

   and the same with `fast-on`. `env.json` records the model, the settings snapshot (no key and no URL),
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