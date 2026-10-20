# See what it did

**Status:** Works.

## What it is

Everything Rune does is recorded, so you can check later what happened and why.

## How to use

1. Run any task first.
2. To watch it, click **Review** on the card that appears when it ends, or see [12 Review a session](12-review-a-session.md).
3. In a terminal run `rune-studio-cli runs` to list recent tasks.
4. Pick an id from the list and run `rune-studio-cli run <id>` to see its steps, or `rune-studio-cli run <id> --calls` to also list every model attempt (retries included).
5. To look at the pictures, open the task's folder: `Documents\Rune Studio\sessions\<date>\<time>-<n>-<name>\`.

## What happens

1. Each task gets its own folder with screenshots, voice clips and a `session.json`.
2. A database (`trace.sqlite3`) holds every run and step: the prompt, the reply, timings, the safety decision, the result and the quality check, and every model call with its raw request and response (images and audio are replaced by their size).
3. A task that was cut off by a crash is listed as "(open)".

## What you will see

A table of runs marked succeeded, stopped, failed or reported_failure. For one run, each step with its timings and outcome.

## How well it works

Measured: all outcome kinds appear, folder names equal run ids, 561 reads during a live task had no failures, and a run takes about 0.15 MB (80 runs were 11 MB of files plus a 270 KB database).

## Limits

- The record cannot be switched off.
- Text you typed is stored, including a spoken password. The titles of open browser tabs are stored too. So are the raw model requests and replies (cut at 64 KB) and the session video.
- Old folders are deleted only if you set a retention period.
- Under heavy load the database may drop rows (not tested).
- The server's token counts are unreliable, so the evaluation estimates them.

## Where it can grow

Hiding secrets, export and delete, automatic post-mortems.

## Features used

Session folders, the SQLite trace, the command-line tool, the Flight Player.