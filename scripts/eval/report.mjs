// Turns one or two results.jsonl files from an eval run into a report, and judges a change against a
// baseline with one rule: success rate not lower, median task time within +10%.
//
//   node scripts/eval/report.mjs <results.jsonl> [--compare <baseline.jsonl>] [--out report.md]
//
// The first file is the run being judged; --compare names the baseline it is judged against. Exit
// code 1 when a comparison's verdict is FAIL. No dependencies.

import { readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

/** The gate's tolerance on the median task duration: more than this much slower fails. */
export const MAX_SLOWER_PERCENT = 10

export function parseResults(text) {
  return text
    .split(/\r?\n/)
    .filter((line) => line.trim() !== '')
    .map((line, index) => {
      try {
        return JSON.parse(line)
      } catch {
        throw new Error(`results line ${String(index + 1)} is not JSON`)
      }
    })
}

/** The middle value (the mean of the two middle ones for an even count); null for none. */
export function median(numbers) {
  if (numbers.length === 0) return null
  const sorted = [...numbers].sort((a, b) => a - b)
  const middle = Math.floor(sorted.length / 2)
  return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2
}

function sum(numbers) {
  return numbers.reduce((total, value) => total + value, 0)
}

/** Aggregates one group of runs. */
export function summarize(rows) {
  const passes = rows.filter((row) => row.passed).length
  const perStepDecide = rows
    .filter((row) => row.steps > 0)
    .map((row) => row.decide_ms_total / row.steps)
  return {
    runs: rows.length,
    passes,
    rate: rows.length === 0 ? null : passes / rows.length,
    medianSteps: median(rows.map((row) => row.steps)),
    medianDuration: median(rows.map((row) => row.duration_ms)),
    medianDecidePerStep: median(perStepDecide),
    totalTokens: sum(rows.map((row) => row.completion_tokens_total)),
  }
}

/** Per-task summaries in the order the tasks first appear, plus the overall summary. */
export function aggregate(rows) {
  const order = []
  const byTask = new Map()
  for (const row of rows) {
    if (!byTask.has(row.task)) {
      byTask.set(row.task, [])
      order.push(row.task)
    }
    byTask.get(row.task).push(row)
  }
  return {
    overall: summarize(rows),
    tasks: order.map((task) => ({ task, ...summarize(byTask.get(task)) })),
  }
}

/**
 * Where the model's own claim and the check differ. For a task that should succeed, `self_reported_success`
 * is compared with `check_passed`. For a task that should fail, `check_passed` only means nothing bad
 * happened, so what is listed is the model claiming success on it.
 */
export function disagreements(rows) {
  const ordinary = rows.filter((row) => row.expect !== 'failure')
  return {
    claimedSuccessCheckFailed: ordinary.filter(
      (row) => row.self_reported_success && !row.check_passed,
    ),
    claimedFailureCheckPassed: ordinary.filter(
      (row) => !row.self_reported_success && row.check_passed,
    ),
    claimedSuccessOnImpossible: rows.filter(
      (row) => row.expect === 'failure' && row.self_reported_success,
    ),
  }
}

/**
 * The gate. `current` and `baseline` are lists of runs. PASS when the success rate is not lower than the
 * baseline's by more than one run's worth (1 / the current number of runs) and the median task duration is
 * not more than 10% higher. With this few repeats it is indicative, not a statistical test.
 */
export function gate(current, baseline) {
  const now = summarize(current)
  const before = summarize(baseline)
  if (now.runs === 0 || before.runs === 0) {
    return { verdict: 'FAIL', reasons: ['one of the two result files has no runs'], now, before }
  }
  const reasons = []
  // passes/runs >= basePasses/baseRuns - 1/runs, kept in integers: passes * baseRuns >= basePasses * runs - baseRuns
  if (now.passes * before.runs < before.passes * now.runs - before.runs) {
    const lost = (before.rate - now.rate) * 100
    reasons.push(
      `success rate ${pct(now.rate)} is lower than the baseline's ${pct(before.rate)} by more than one run's worth (${lost.toFixed(1)} points, one run is ${(100 / now.runs).toFixed(1)} points)`,
    )
  }
  // median > baseline * 1.10, in integers: median * 100 > baseline * (100 + 10)
  if (
    before.medianDuration !== null &&
    now.medianDuration * 100 > before.medianDuration * (100 + MAX_SLOWER_PERCENT)
  ) {
    const slower = (now.medianDuration / before.medianDuration - 1) * 100
    reasons.push(
      `median task duration ${seconds(now.medianDuration)} is ${slower.toFixed(1)}% above the baseline's ${seconds(before.medianDuration)} (limit ${String(MAX_SLOWER_PERCENT)}%)`,
    )
  }
  return { verdict: reasons.length === 0 ? 'PASS' : 'FAIL', reasons, now, before }
}

/** The words a run's `verdict` can hold, in the order the matrix lists them. */
export const VERDICTS = ['verified', 'contradicted', 'unsure', 'error', 'skipped', 'timeout']

/**
 * The independent check against the machine's checkers. Only runs that have a verdict and should
 * succeed count (a task that should fail has no "check passed" to compare with). Returns the confusion
 * matrix (verdict by `check_passed`) and four numbers:
 *  - agreement among decisive verdicts: (verified and the check passed, or contradicted and it failed) over verified plus contradicted;
 *  - false contradictions: contradicted while the check passed;
 *  - missed failures: verified while the check failed;
 *  - unsure rate: unsure over verified plus contradicted plus unsure (runs that were really checked).
 */
export function verifierStats(rows) {
  const counted = rows.filter(
    (row) => row.verdict !== undefined && row.verdict !== null && row.expect !== 'failure',
  )
  const matrix = Object.fromEntries(VERDICTS.map((verdict) => [verdict, { pass: 0, fail: 0 }]))
  for (const row of counted) {
    const cell = (matrix[row.verdict] ??= { pass: 0, fail: 0 })
    if (row.check_passed) cell.pass++
    else cell.fail++
  }
  const total = (verdict) => matrix[verdict].pass + matrix[verdict].fail
  const decisive = total('verified') + total('contradicted')
  const agreeing = matrix.verified.pass + matrix.contradicted.fail
  const judged = decisive + total('unsure')
  return {
    runs: counted.length,
    matrix,
    decisive,
    agreement: decisive === 0 ? null : agreeing / decisive,
    falseContradictions: matrix.contradicted.pass,
    missedFailures: matrix.verified.fail,
    unsureRate: judged === 0 ? null : total('unsure') / judged,
  }
}

/** Which check modes a file's runs were made in (a file from before the verifier has none: that is `off`). */
export function modesOf(rows) {
  return [...new Set(rows.map((row) => row.verify_mode ?? 'off'))].sort((a, b) =>
    a.localeCompare(b),
  )
}

/**
 * What the check cost in time: the median task duration and median steps now against the baseline.
 * Blocking mode's check happens inside the task, so it is in `duration_ms`; a background check is not.
 */
export function overhead(current, baseline) {
  const now = summarize(current)
  const before = summarize(baseline)
  const durationPercent =
    now.medianDuration === null || before.medianDuration === null || before.medianDuration === 0
      ? null
      : (now.medianDuration / before.medianDuration - 1) * 100
  return {
    currentModes: modesOf(current),
    baselineModes: modesOf(baseline),
    medianDuration: {
      now: now.medianDuration,
      before: before.medianDuration,
      percent: durationPercent,
    },
    medianSteps: {
      now: now.medianSteps,
      before: before.medianSteps,
      delta:
        now.medianSteps === null || before.medianSteps === null
          ? null
          : now.medianSteps - before.medianSteps,
    },
  }
}

function pct(rate) {
  return rate === null ? '-' : `${(rate * 100).toFixed(1)}%`
}

function seconds(ms) {
  return ms === null ? '-' : `${(ms / 1000).toFixed(1)}s`
}

function num(value, digits = 1) {
  return value === null ? '-' : String(Number(value.toFixed(digits)))
}

function delta(now, before, format) {
  if (now === null || before === null) return '-'
  const difference = now - before
  return `${difference >= 0 ? '+' : '-'}${format(Math.abs(difference))}`
}

function table(header, rows) {
  const line = (cells) => `| ${cells.join(' | ')} |`
  return [line(header), line(header.map(() => '---')), ...rows.map(line)].join('\n')
}

function runList(rows) {
  if (rows.length === 0) return '_none_'
  return table(
    ['task', 'repeat', 'run', 'outcome'],
    rows.map((row) => [row.task, String(row.repeat), row.run_id, row.outcome]),
  )
}

/** The whole report as Markdown. */
export function render(current, baseline) {
  const now = aggregate(current)
  const out = ['# Eval report', '']
  let verdict = null
  if (baseline) {
    verdict = gate(current, baseline)
    out.push(`## Gate: ${verdict.verdict}`, '')
    out.push(
      ...(verdict.reasons.length === 0
        ? [
            'Success rate is not lower than the baseline by more than one run, and the median task duration is within +10%.',
          ]
        : verdict.reasons.map((reason) => `- ${reason}`)),
    )
    out.push('', '_With this few repeats the gate is indicative, not a statistical test._', '')
  }
  if (baseline) {
    const cost = overhead(current, baseline)
    const percent =
      cost.medianDuration.percent === null
        ? '-'
        : `${cost.medianDuration.percent >= 0 ? '+' : ''}${cost.medianDuration.percent.toFixed(1)}%`
    const steps =
      cost.medianSteps.delta === null
        ? '-'
        : `${cost.medianSteps.delta >= 0 ? '+' : ''}${String(Number(cost.medianSteps.delta.toFixed(1)))}`
    out.push('## Verifier overhead (this run against the baseline)', '')
    out.push(
      `Check modes: this run \`${cost.currentModes.join(', ')}\`, baseline \`${cost.baselineModes.join(', ')}\`.`,
      '',
    )
    out.push(
      table(
        ['', 'now', 'baseline', 'change'],
        [
          [
            'median duration',
            seconds(cost.medianDuration.now),
            seconds(cost.medianDuration.before),
            percent,
          ],
          ['median steps', num(cost.medianSteps.now), num(cost.medianSteps.before), steps],
        ],
      ),
      '',
    )
    out.push(
      'A blocking check runs inside the task, so it is in these durations. A background check runs after it and is not.',
      '',
    )
  }
  const base = baseline ? aggregate(baseline) : null
  const o = now.overall
  out.push('## Overall', '')
  out.push(
    table(
      base ? ['', 'now', 'baseline', 'change'] : ['', 'value'],
      [
        ['runs', o.runs, base?.overall.runs],
        ['passed', o.passes, base?.overall.passes],
        [
          'success rate',
          pct(o.rate),
          base && pct(base.overall.rate),
          base && delta(o.rate, base.overall.rate, (value) => `${(value * 100).toFixed(1)} points`),
        ],
        [
          'median duration',
          seconds(o.medianDuration),
          base && seconds(base.overall.medianDuration),
          base && delta(o.medianDuration, base.overall.medianDuration, seconds),
        ],
        [
          'median steps',
          num(o.medianSteps),
          base && num(base.overall.medianSteps),
          base && delta(o.medianSteps, base.overall.medianSteps, num),
        ],
        [
          'completion tokens',
          o.totalTokens,
          base?.overall.totalTokens,
          base && delta(o.totalTokens, base.overall.totalTokens, String),
        ],
      ].map((cells) =>
        (base ? cells : cells.slice(0, 2)).map((cell) =>
          cell === undefined || cell === null ? '-' : String(cell),
        ),
      ),
    ),
    '',
  )
  out.push('## Per task', '')
  const baseByTask = new Map((base?.tasks ?? []).map((task) => [task.task, task]))
  out.push(
    table(
      base
        ? [
            'task',
            'runs',
            'passed',
            'pass rate',
            'baseline rate',
            'median steps',
            'median duration',
            'baseline duration',
            'median decide/step',
          ]
        : [
            'task',
            'runs',
            'passed',
            'pass rate',
            'median steps',
            'median duration',
            'median decide/step',
          ],
      now.tasks.map((task) => {
        const before = baseByTask.get(task.task)
        const common = [task.task, String(task.runs), String(task.passes), pct(task.rate)]
        const tail = [num(task.medianSteps), seconds(task.medianDuration)]
        const decide = seconds(task.medianDecidePerStep)
        return base
          ? [
              ...common,
              before ? pct(before.rate) : '-',
              ...tail,
              before ? seconds(before.medianDuration) : '-',
              decide,
            ]
          : [...common, ...tail, decide]
      }),
    ),
    '',
  )
  const split = disagreements(current)
  out.push('## Where the model and the check disagree', '')
  out.push(
    'This is the label data for the verifier. `self_reported_success` is true when the run ended `succeeded`; `check_passed` is what the machine says.',
    '',
  )
  out.push(
    `### Claimed success, check failed (${String(split.claimedSuccessCheckFailed.length)})`,
    '',
    runList(split.claimedSuccessCheckFailed),
    '',
  )
  out.push(
    `### Claimed failure, check passed (${String(split.claimedFailureCheckPassed.length)})`,
    '',
    runList(split.claimedFailureCheckPassed),
    '',
  )
  out.push(
    `### Claimed success on a task that should fail (${String(split.claimedSuccessOnImpossible.length)})`,
    '',
    runList(split.claimedSuccessOnImpossible),
    '',
  )
  out.push('## Verifier vs the checkers', '')
  const stats = verifierStats(current)
  if (stats.runs === 0) {
    out.push(
      '_No verdicts recorded (the check was off, or no run that should succeed has one)._',
      '',
    )
  } else {
    out.push(
      `The independent check against \`check_passed\`, for the ${String(stats.runs)} runs that should succeed and have a verdict.`,
      '',
    )
    out.push(
      table(
        ['verdict', 'check passed', 'check failed', 'total'],
        VERDICTS.map((verdict) => [
          verdict,
          String(stats.matrix[verdict].pass),
          String(stats.matrix[verdict].fail),
          String(stats.matrix[verdict].pass + stats.matrix[verdict].fail),
        ]),
      ),
      '',
    )
    out.push(
      table(
        ['', 'value'],
        [
          [
            'agreement among decisive verdicts',
            stats.agreement === null
              ? '-'
              : `${pct(stats.agreement)} (${String(stats.decisive)} decisive)`,
          ],
          ['false contradictions (contradicted, check passed)', String(stats.falseContradictions)],
          ['missed failures (verified, check failed)', String(stats.missedFailures)],
          ['unsure rate', pct(stats.unsureRate)],
        ],
      ),
      '',
    )
  }
  const problems = current.filter((row) => row.error)
  if (problems.length > 0) {
    out.push(
      `## Runs with problems (${String(problems.length)})`,
      '',
      table(
        ['task', 'repeat', 'error'],
        problems.map((row) => [row.task, String(row.repeat), String(row.error)]),
      ),
      '',
    )
  }
  return { markdown: out.join('\n'), verdict }
}

function main(argv) {
  const args = argv.slice(2)
  let file
  let compare
  let out
  for (let index = 0; index < args.length; index++) {
    if (args[index] === '--compare') compare = args[++index]
    else if (args[index] === '--out') out = args[++index]
    else file = args[index]
  }
  if (
    !file ||
    args.includes('--compare') !== (compare !== undefined) ||
    args.includes('--out') !== (out !== undefined)
  ) {
    console.error(
      'usage: node scripts/eval/report.mjs <results.jsonl> [--compare <baseline.jsonl>] [--out report.md]',
    )
    return 2
  }
  const current = parseResults(readFileSync(file, 'utf8'))
  const baseline = compare ? parseResults(readFileSync(compare, 'utf8')) : null
  const { markdown, verdict } = render(current, baseline)
  if (out) writeFileSync(out, `${markdown}\n`)
  console.log(markdown)
  return verdict?.verdict === 'FAIL' ? 1 : 0
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv)
}