#!/usr/bin/env bash
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
native() { if command -v cygpath >/dev/null 2>&1; then cygpath -m "$1"; else printf '%s' "$1"; fi; }

suite="$here/suite.desktop-15.json"
out=""
repeats=0
fast_checks="suite"
no_build=false
dry_run=false
while [ $# -gt 0 ]; do
  case "$1" in
    --suite) suite="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --repeats) repeats="$2"; shift 2 ;;
    --fast-checks) fast_checks="$2"; shift 2 ;;
    --no-build) no_build=true; shift ;;
    --dry-run) dry_run=true; shift ;;
    *) echo "Unknown option '$1'" >&2; exit 1 ;;
  esac
done
case "$fast_checks" in on | off | suite) ;; *) echo "--fast-checks must be on, off or suite" >&2; exit 1 ;; esac
case "$repeats" in '' | *[!0-9]*) echo "--repeats must be a whole number" >&2; exit 1 ;; esac
[ -f "$suite" ] || { echo "No suite at $suite" >&2; exit 1; }
command -v node >/dev/null 2>&1 || { echo 'node is required to read the suite. Install Node 22+.' >&2; exit 1; }

plan() {
  node - "$(native "$suite")" "$repeats" "$fast_checks" "${1:-}" <<'JS'
const fs = require('fs')
const [file, repeatsArg, fastChecks, copyTo] = process.argv.slice(2)
const fail = (message) => {
  console.error(message)
  process.exit(1)
}
const allowedToClose = ['notepad.exe', 'calculatorapp.exe', 'calculator.exe', 'systemsettings.exe', 'mspaint.exe']
const spec = JSON.parse(fs.readFileSync(file, 'utf8').replace(/^﻿/, ''))
if (spec.version !== 1) fail(`The suite is version ${spec.version}; this runner reads version 1`)
if (!Array.isArray(spec.tasks) || spec.tasks.length === 0) fail('The suite has no tasks')
const ids = new Set()
for (const task of spec.tasks) {
  if (!/^[a-z0-9-]+$/.test(task.id ?? '')) fail(`Task id '${task.id}' must be lower case letters, digits and dashes`)
  if (ids.has(task.id)) fail(`Task id '${task.id}' is used twice`)
  ids.add(task.id)
  for (const step of [...[task.setup ?? []].flat(), ...[task.teardown ?? []].flat()]) {
    if (step && step.kind === 'kill_process' && !allowedToClose.includes(String(step.name).toLowerCase()))
      fail(`Task ${task.id}: '${step.name}' is not a program a suite may close`)
  }
}
const repeats = Number(repeatsArg) > 0 ? Number(repeatsArg) : spec.repeats ? Math.trunc(spec.repeats) : 1
const timeout = spec.defaults?.timeout_secs ? Math.trunc(spec.defaults.timeout_secs) : 240
const configured = spec.config?.fast_checks
const fast =
  fastChecks === 'on' || fastChecks === 'off'
    ? fastChecks
    : configured === undefined || configured === null
      ? "the user's setting"
      : configured
        ? 'on'
        : 'off'
const runs = spec.tasks.length * repeats
console.log(`Suite:       ${spec.name} (${spec.tasks.length} tasks x ${repeats} repeats = ${runs} runs)`)
console.log(`Fast checks: ${fast}`)
console.log(
  `Longest run: up to ${(runs * timeout) / 60} minutes if every run times out (${timeout} s each); a normal run is a few minutes in all`,
)
for (const task of spec.tasks) console.log(`  - ${task.id}${task.expect === 'failure' ? ' (expect failure)' : ''}`)
if (copyTo) {
  spec.repeats = repeats
  if (fastChecks !== 'suite') spec.config = { ...spec.config, fast_checks: fastChecks === 'on' }
  fs.writeFileSync(copyTo, JSON.stringify(spec, null, 2))
}
JS
}

if [ "$dry_run" = true ]; then
  plan || exit 1
  echo 'Dry run: nothing was built or started.'
  exit 0
fi

if command -v tasklist >/dev/null 2>&1; then
  running="$(tasklist //FI "IMAGENAME eq rune-studio.exe" 2>/dev/null | grep -ci 'rune-studio.exe')"
else
  running="$(pgrep -cx rune-studio 2>/dev/null)"
fi
if [ "${running:-0}" -gt 0 ]; then
  echo 'Close Rune Studio first: the evaluation starts its own copy.' >&2
  exit 1
fi

[ -n "$out" ] || out="$root/eval-out/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out" || exit 1
out="$(cd "$out" && pwd)"
suite_copy="$out/suite.json"
plan "$(native "$suite_copy")" || exit 1
rm -rf "$out/fixtures"
cp -R "$here/fixtures" "$out/fixtures" || exit 1

exe="$root/src-tauri/target/debug/rune-studio"
[ -f "$exe.exe" ] || [ "${OS:-}" = "Windows_NT" ] && exe="$exe.exe"
if [ "$no_build" = false ]; then
  RUNE_GIT_COMMIT="$(git -C "$root" rev-parse --short HEAD 2>/dev/null)"
  export RUNE_GIT_COMMIT
  (cd "$root/src-tauri" && cargo build --features 'eval,agent-bridge') || { echo 'cargo build failed' >&2; exit 1; }
fi
[ -f "$exe" ] || { echo "No app at $exe" >&2; exit 1; }

echo "Output: $out"
echo 'Starting. Do not touch the mouse or keyboard. Stop with Ctrl+Alt+Esc.'
RUNE_EVAL_SUITE="$(native "$suite_copy")" RUNE_EVAL_OUT="$(native "$out")" "$exe"
code=$?

if [ "$code" -eq 2 ]; then
  echo 'The evaluation did not start:'
  cat "$out/preflight.txt" 2>/dev/null
  exit 2
fi
if [ "$code" -ne 0 ]; then
  echo "The evaluation finished but a setup or teardown step failed (exit code $code). See the 'error' column of results.jsonl."
fi

results="$out/results.jsonl"
[ -f "$results" ] || { echo "No results.jsonl in $out" >&2; exit 1; }
echo "Results: $results"
node "$(native "$here/report.mjs")" "$(native "$results")" --out "$(native "$out/report.md")"
echo "Report:  $out/report.md"
echo 'Close the fixture pages the run left open in your browser.'
exit "$code"
