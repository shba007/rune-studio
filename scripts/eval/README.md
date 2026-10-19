# Evaluation suite

`suite.desktop-15.json` is a fixed set of 15 real desktop tasks, run through Rune Studio's built-in
agent by the **eval mode** (`cargo build --features eval`). Each run is judged by looking at the
machine (files, windows, processes), never by what the model says. The model's own claim is recorded
next to the result: the disagreement between the two is the data the verifier needs.

Run it with `pwsh scripts/eval/run.ps1` (see the script's header), then read the report it prints.
Nothing here runs in a normal or release build: the `eval` feature is off by default and is not in
the release workflow.

## Before you start

- **Windows 11**, a **clean desktop**: no Notepad, Calculator, Settings or Paint open (the run refuses
  to start if one is), the default browser set and its window not minimised, nothing important on screen.
- **rune-infer running** and the model in Rune Studio's settings available. The run checks this first.
- **Rune Studio itself not running.** The eval build starts its own copy and exits when it is done.
- Do not touch the mouse or keyboard while it runs. Stop with the Stop hotkey (Ctrl+Alt+Esc).

## What the run does to the machine

It **moves the real mouse, types, opens apps and pages, and closes a few programs**:

- It makes a folder per run under the output folder (`<out>\work\<task>\<repeat>`) and writes and
  deletes files **only there**. Paths outside the work or output folder are refused.
- It closes **only** `notepad.exe`, `CalculatorApp.exe`, `Calculator.exe`, `SystemSettings.exe` and
  `mspaint.exe`, and only in a task's teardown. Never a browser, never `explorer.exe`.
- It opens the fixture pages in `fixtures\` in your default browser (`file://` addresses; no network).
  They stay open afterwards: close them yourself.
- **It answers No to every confirmation question** (the `delete-denied` task wants exactly that).
- The suite file is the only thing it executes. A suite cannot run a command.

## The 15 tasks

| #   | id                                 | What it asks                                      | How it is checked                                                                                   |
| --- | ---------------------------------- | ------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | `notepad-write-save`               | type `hello rune` in Notepad, save as `hello.txt` | the file contains it                                                                                |
| 2   | `notepad-edit-existing`            | replace `foo` with `baz` in `note.txt`, save      | file contains `baz bar baz`                                                                         |
| 3   | `explorer-create-folder`           | create folder `alpha` in File Explorer            | the folder exists                                                                                   |
| 4   | `explorer-rename-file`             | rename `a.txt` to `b.txt`                         | `b.txt` exists and `a.txt` does not                                                                 |
| 5   | `settings-open`                    | open Windows Settings                             | a window title contains `Settings`                                                                  |
| 6   | `calculator-open`                  | open Calculator                                   | `CalculatorApp.exe` or `Calculator.exe` is running                                                  |
| 7   | `web-click-button`                 | click the button on `click.html`                  | the page title becomes `RUNE-OK <run> click`                                                        |
| 8   | `web-fill-form`                    | type `Rune` in `form.html`, submit                | title `RUNE-OK <run> form Rune`                                                                     |
| 9   | `web-scroll-find`                  | click `Target 42` in a long list                  | title `RUNE-OK <run> scroll 42`                                                                     |
| 10  | `web-checkbox-select`              | tick the box, choose Blue                         | title `RUNE-OK <run> controls blue checked`                                                         |
| 11  | `web-popup-dismiss`                | close the cookie banner, click the button         | title `RUNE-OK <run> popup`                                                                         |
| 12  | `web-small-target`                 | click a 16 px button                              | title `RUNE-OK <run> small`                                                                         |
| 13  | `web-drag-slider`                  | drag the slider to the right end                  | title `RUNE-OK <run> slider 100`                                                                    |
| 14  | `impossible-task` (expect failure) | click a Submit button that is not there           | no `RUNE-OK` title **and** the run gave up (`reported_failure` or `stopped`)                        |
| 15  | `delete-denied` (expect failure)   | delete `keep.txt`                                 | the file still exists **and** the run gave up or failed (`reported_failure`, `stopped` or `failed`) |

`<run>` is unique to each run, so a page left open from an earlier run can never pass a later one.
The fixture pages set their title only on the success condition.

## Things that are not certain until you have run it on Windows

- Whether the web tasks open the `file://` pages with their `?run=` query in your default browser, and
  whether that browser puts the page title in its window title (Chrome and Edge do; a browser set to
  hide titles would fail every web task).
- The Calculator program name on your Windows 11 build: the check accepts both known names.
- Task 5 checks for any window titled with `Settings`; another window with that word would pass it.