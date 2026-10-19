# Runs the evaluation suite on this machine and writes a report.
#
#   pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks off
#   pwsh scripts/eval/run.ps1 -DryRun                      # only validates the suite and prints the plan
#
# Read scripts/eval/README.md first: it MOVES THE REAL MOUSE, TYPES, OPENS APPS AND PAGES, and closes a
# few programs (Notepad, Calculator, Settings, Paint) while it runs. Needs Windows 11, a clean desktop,
# a running rune-infer, and Rune Studio itself closed.
#
# -Suite       the suite file (default scripts/eval/suite.desktop-15.json)
# -Out         the output folder (default eval-out/<timestamp> under the repository)
# -Repeats     overrides the suite's repeats
# -FastChecks  on | off | suite  (suite keeps what the suite file says; default)
# -NoBuild     use the eval build that is already in src-tauri/target/debug
# -DryRun      validate and print the plan, change nothing
#
# It builds the app with `--features eval,agent-bridge`, runs it with RUNE_EVAL_SUITE and
# RUNE_EVAL_OUT set (the app exits by itself when done), then runs report.mjs. The eval feature is
# never part of a release build.

param(
    [string]$Suite = (Join-Path $PSScriptRoot 'suite.desktop-15.json'),
    [string]$Out = '',
    [int]$Repeats = 0,
    [ValidateSet('on', 'off', 'suite')][string]$FastChecks = 'suite',
    [switch]$NoBuild,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$allowedToClose = @('notepad.exe', 'calculatorapp.exe', 'calculator.exe', 'systemsettings.exe', 'mspaint.exe')

if (-not (Test-Path $Suite)) { throw "No suite at $Suite" }
$spec = Get-Content $Suite -Raw | ConvertFrom-Json

# The app checks the suite itself (and refuses to start on a bad one); this catches the common mistakes early.
function Test-Suite($spec) {
    if ($spec.version -ne 1) { throw "The suite is version $($spec.version); this runner reads version 1" }
    if (-not $spec.tasks -or $spec.tasks.Count -eq 0) { throw 'The suite has no tasks' }
    $ids = @{}
    foreach ($task in $spec.tasks) {
        if ($task.id -notmatch '^[a-z0-9-]+$') { throw "Task id '$($task.id)' must be lower case letters, digits and dashes" }
        if ($ids.ContainsKey($task.id)) { throw "Task id '$($task.id)' is used twice" }
        $ids[$task.id] = $true
        foreach ($step in @($task.setup) + @($task.teardown)) {
            if ($step -and $step.kind -eq 'kill_process' -and ($allowedToClose -notcontains $step.name.ToLower())) {
                throw "Task $($task.id): '$($step.name)' is not a program a suite may close"
            }
        }
    }
}
Test-Suite $spec

$repeats = if ($Repeats -gt 0) { $Repeats } elseif ($spec.repeats) { [int]$spec.repeats } else { 1 }
$timeout = if ($spec.defaults.timeout_secs) { [int]$spec.defaults.timeout_secs } else { 240 }
$fast = switch ($FastChecks) { 'on' { 'on' } 'off' { 'off' } default { if ($null -eq $spec.config.fast_checks) { "the user's setting" } elseif ($spec.config.fast_checks) { 'on' } else { 'off' } } }
$runs = $spec.tasks.Count * $repeats
"Suite:       $($spec.name) ($($spec.tasks.Count) tasks x $repeats repeats = $runs runs)"
"Fast checks: $fast"
"Longest run: up to $($runs * $timeout / 60) minutes if every run times out ($timeout s each); a normal run is a few minutes in all"
foreach ($task in $spec.tasks) {
    $expect = if ($task.expect -eq 'failure') { ' (expect failure)' } else { '' }
    "  - $($task.id)$expect"
}
if ($DryRun) { 'Dry run: nothing was built or started.'; return }

if (Get-Process -Name 'rune-studio' -ErrorAction SilentlyContinue) { throw 'Close Rune Studio first: the evaluation starts its own copy.' }

if (-not $Out) { $Out = Join-Path $root ('eval-out\' + (Get-Date -Format 'yyyyMMdd-HHmmss')) }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path

# The app reads its fixture pages from next to the suite file, so the run's copy of both goes in the output folder.
$spec | Add-Member -NotePropertyName repeats -NotePropertyValue $repeats -Force
if ($FastChecks -ne 'suite') {
    if (-not $spec.config) { $spec | Add-Member -NotePropertyName config -NotePropertyValue ([pscustomobject]@{}) }
    $spec.config | Add-Member -NotePropertyName fast_checks -NotePropertyValue ($FastChecks -eq 'on') -Force
}
$suiteCopy = Join-Path $Out 'suite.json'
$spec | ConvertTo-Json -Depth 20 | Set-Content -Path $suiteCopy -Encoding UTF8
Copy-Item -Recurse -Force (Join-Path $PSScriptRoot 'fixtures') (Join-Path $Out 'fixtures')

$exe = Join-Path $root 'src-tauri\target\debug\rune-studio.exe'
if (-not $NoBuild) {
    $env:RUNE_GIT_COMMIT = (git -C $root rev-parse --short HEAD 2>$null)
    Push-Location (Join-Path $root 'src-tauri')
    try { cargo build --features 'eval,agent-bridge'; if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' } } finally { Pop-Location }
}
if (-not (Test-Path $exe)) { throw "No app at $exe" }

"Output: $Out"
'Starting. Do not touch the mouse or keyboard. Stop with Ctrl+Alt+Esc.'
$env:RUNE_EVAL_SUITE = $suiteCopy
$env:RUNE_EVAL_OUT = $Out
$app = Start-Process -FilePath $exe -PassThru
$app.WaitForExit()
Remove-Item Env:RUNE_EVAL_SUITE, Env:RUNE_EVAL_OUT -ErrorAction SilentlyContinue

$code = $app.ExitCode
if ($code -eq 2) {
    'The evaluation did not start:'
    Get-Content (Join-Path $Out 'preflight.txt') -ErrorAction SilentlyContinue
    exit 2
}
if ($code -ne 0) { "The evaluation finished but a setup or teardown step failed (exit code $code). See the 'error' column of results.jsonl." }

$results = Join-Path $Out 'results.jsonl'
if (-not (Test-Path $results)) { throw "No results.jsonl in $Out" }
"Results: $results"
if (Get-Command node -ErrorAction SilentlyContinue) {
    node (Join-Path $PSScriptRoot 'report.mjs') $results --out (Join-Path $Out 'report.md')
    "Report:  $(Join-Path $Out 'report.md')"
} else {
    'node is not installed, so no report was made. Install Node 22+ and run: node scripts/eval/report.mjs <results.jsonl>'
}
'Close the fixture pages the run left open in your browser.'
exit $code
