# 08 See what it did

**Status:** Works.

## What it is
Everything Rune does is recorded, so you can check later what happened and why.

## Try it
```
rune-studio-cli runs
rune-studio-cli run <id>
```

## What happens
- Each task gets a folder in `Documents/Rune Studio/sessions/<date>/<time>-<n>-<name>/` with screenshots, voice clips and a `session.json`.
- A database (`trace.sqlite3`) holds every run and step: the prompt, the reply, timings, the safety decision, the result and the quality check.
- `runs` lists tasks as succeeded, stopped, failed, reported_failure or (open) if the app died mid-task.

## What you will see
A table of runs, then for one run each step with timings and outcomes.

## How well it works
Measured: all outcome kinds appear; folder names equal run ids; 561 reads during a live task with no failures; about 0.15 MB per run (80 runs = 11 MB of files plus a 270 KB database).

## Limits
- The record cannot be switched off.
- Text you typed is stored, including a spoken password. Open browser tab titles are stored too.
- Old folders are deleted only if you set a retention period.
- Under heavy load the database may drop rows (not tested).
- The server's token counts are unreliable; the evaluation estimates them.

## Where it can grow
Hiding secrets, export and delete, a window to browse runs, automatic post-mortems (see [11](11-idle-reflection.md)).

## Features used
Session folders, the SQLite trace, the CLI.
