import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'

import {
  aggregate,
  disagreements,
  gate,
  median,
  parseResults,
  render,
  summarize,
} from '../report.mjs'

const here = fileURLToPath(new URL('.', import.meta.url))
const load = (name) => parseResults(readFileSync(here + name, 'utf8'))
const script = fileURLToPath(new URL('../report.mjs', import.meta.url))

test('the median of an odd count is the middle value, of an even count the mean of the two', () => {
  assert.equal(median([3, 1, 2]), 2)
  assert.equal(median([4, 1, 3, 2]), 2.5)
  assert.equal(median([7]), 7)
  assert.equal(median([]), null)
  assert.equal(median([10, 10, 10, 10]), 10)
})

test('runs are aggregated per task and overall', () => {
  const { overall, tasks } = aggregate(load('agg.jsonl'))
  assert.deepEqual(
    tasks.map((task) => [task.task, task.runs, task.passes]),
    [
      ['t1', 3, 2],
      ['t2', 2, 1],
      ['t3', 1, 0],
    ],
  )
  const [t1, t2] = tasks
  assert.equal(t1.rate, 2 / 3)
  assert.equal(t1.medianDuration, 200, 'odd count: the middle of 100, 300, 200')
  assert.equal(t1.medianSteps, 4)
  assert.equal(t1.medianDecidePerStep, 300, 'per-step decide time: 200, 400, 300 -> 300')
  assert.equal(t1.totalTokens, 60)
  assert.equal(t2.medianDuration, 1500, 'even count: the mean of 1000 and 2000')
  assert.equal(t2.medianDecidePerStep, 200, 'a run with no steps is left out: only 1000 / 5')
  assert.equal(overall.runs, 6)
  assert.equal(overall.passes, 3)
  assert.equal(
    overall.medianDuration,
    400,
    'the middle two of 100, 200, 300, 500, 1000, 2000 are 300 and 500',
  )
  assert.equal(overall.totalTokens, 105)
})

test('the disagreement table splits claimed success from claimed failure', () => {
  const split = disagreements(load('agg.jsonl'))
  assert.deepEqual(
    split.claimedSuccessCheckFailed.map((row) => row.run_id),
    ['t1-3'],
  )
  assert.deepEqual(
    split.claimedFailureCheckPassed.map((row) => row.run_id),
    ['t2-1'],
  )
  assert.deepEqual(
    split.claimedSuccessOnImpossible.map((row) => row.run_id),
    ['t3-1'],
    'a task that should fail is judged on the claim alone, not on check_passed',
  )
})

test('the gate: equal runs pass', () => {
  const result = gate(load('same.jsonl'), load('base.jsonl'))
  assert.equal(result.verdict, 'PASS')
  assert.deepEqual(result.reasons, [])
})

test('the gate: one run lower is within the tolerance and passes', () => {
  const result = gate(load('one-lower.jsonl'), load('base.jsonl'))
  assert.equal(summarize(load('one-lower.jsonl')).passes, 13)
  assert.equal(result.verdict, 'PASS')
})

test('the gate: a real success drop fails and says why', () => {
  const result = gate(load('drop.jsonl'), load('base.jsonl'))
  assert.equal(result.verdict, 'FAIL')
  assert.equal(result.reasons.length, 1)
  assert.match(result.reasons[0], /success rate 73\.3% is lower than the baseline's 93\.3%/)
})

test('the gate: 12% slower fails, exactly 10% slower passes', () => {
  const slow = gate(load('slow.jsonl'), load('base.jsonl'))
  assert.equal(slow.verdict, 'FAIL')
  assert.match(
    slow.reasons.join(' '),
    /median task duration 11\.2s is 12\.0% above the baseline's 10\.0s/,
  )
  assert.equal(gate(load('edge-slow.jsonl'), load('base.jsonl')).verdict, 'PASS')
})

test('the gate fails when there is nothing to compare', () => {
  assert.equal(gate([], load('base.jsonl')).verdict, 'FAIL')
  assert.equal(gate(load('base.jsonl'), []).verdict, 'FAIL')
})

test('the report names the gate verdict at the top and says it is indicative', () => {
  const { markdown, verdict } = render(load('drop.jsonl'), load('base.jsonl'))
  assert.equal(verdict.verdict, 'FAIL')
  assert.ok(markdown.indexOf('## Gate: FAIL') < markdown.indexOf('## Overall'))
  assert.match(markdown, /indicative, not a statistical test/)
  assert.match(markdown, /\| e \| 3 \| 0 \| 0\.0% \|/, 'a per-task row with the baseline column')
  const alone = render(load('agg.jsonl'), null)
  assert.equal(alone.verdict, null)
  assert.ok(!alone.markdown.includes('## Gate'))
  assert.match(alone.markdown, /Claimed success, check failed \(1\)/)
  assert.match(alone.markdown, /Runs with problems \(1\)/)
  assert.match(alone.markdown, /setup: boom/)
})

test('the command line exits 1 on FAIL, 0 on PASS and 2 on bad arguments', () => {
  const run = (...args) => spawnSync(process.execPath, [script, ...args], { encoding: 'utf8' })
  assert.equal(run(here + 'same.jsonl', '--compare', here + 'base.jsonl').status, 0)
  const failed = run(here + 'slow.jsonl', '--compare', here + 'base.jsonl')
  assert.equal(failed.status, 1)
  assert.match(failed.stdout, /## Gate: FAIL/)
  assert.equal(run().status, 2)
  assert.equal(run(here + 'same.jsonl', '--compare').status, 2)
})

test('a line that is not JSON is reported with its number', () => {
  assert.throws(() => parseResults('{"a":1}\nnot json\n'), /line 2/)
  assert.deepEqual(parseResults('\n{"a":1}\n\n'), [{ a: 1 }])
})
test('the verifier is compared with the checkers: the matrix and the four numbers', async () => {
  const { VERDICTS, verifierStats } = await import('../report.mjs')
  const stats = verifierStats(load('verdicts.jsonl'))
  // 13 rows: one has no verdict (check off) and one should fail, so 11 count.
  assert.equal(stats.runs, 11)
  assert.deepEqual(stats.matrix.verified, { pass: 2, fail: 1 })
  assert.deepEqual(stats.matrix.contradicted, { pass: 1, fail: 2 })
  assert.deepEqual(stats.matrix.unsure, { pass: 1, fail: 1 })
  assert.deepEqual(stats.matrix.error, { pass: 1, fail: 0 })
  assert.deepEqual(stats.matrix.skipped, { pass: 0, fail: 1 })
  assert.deepEqual(
    stats.matrix.timeout,
    { pass: 1, fail: 0 },
    'a verdict the harness gave up waiting for is its own row',
  )
  assert.equal(stats.decisive, 6, 'verified plus contradicted')
  assert.equal(
    stats.agreement,
    4 / 6,
    'verified and passed (2) plus contradicted and failed (2), of 6',
  )
  assert.equal(stats.falseContradictions, 1)
  assert.equal(stats.missedFailures, 1)
  assert.equal(stats.unsureRate, 2 / 8, 'unsure over verified, contradicted and unsure')
  assert.deepEqual(VERDICTS, ['verified', 'contradicted', 'unsure', 'error', 'skipped', 'timeout'])
})

test('without verdicts the verifier numbers are empty, not zero', async () => {
  const { verifierStats } = await import('../report.mjs')
  const stats = verifierStats(load('base.jsonl'))
  assert.equal(stats.runs, 0)
  assert.equal(stats.agreement, null)
  assert.equal(stats.unsureRate, null)
  assert.equal(stats.falseContradictions, 0)
  assert.match(render(load('base.jsonl'), null).markdown, /No verdicts recorded/)
  // Only a decided mix has an agreement; a file of "unsure" alone has none.
  assert.equal(
    verifierStats(load('verdicts.jsonl').filter((row) => row.verdict === 'unsure')).agreement,
    null,
  )
})

test('the report shows the matrix and the four numbers', () => {
  const { markdown } = render(load('verdicts.jsonl'), null)
  assert.match(markdown, /## Verifier vs the checkers/)
  assert.match(markdown, /\| verified \| 2 \| 1 \| 3 \|/)
  assert.match(markdown, /\| contradicted \| 1 \| 2 \| 3 \|/)
  assert.match(markdown, /\| timeout \| 1 \| 0 \| 1 \|/)
  assert.match(markdown, /agreement among decisive verdicts \| 66\.7% \(6 decisive\)/)
  assert.match(markdown, /false contradictions \(contradicted, check passed\) \| 1 \|/)
  assert.match(markdown, /missed failures \(verified, check failed\) \| 1 \|/)
  assert.match(markdown, /unsure rate \| 25\.0% \|/)
})

test('compare prints what blocking mode costs next to the gate', async () => {
  const { overhead } = await import('../report.mjs')
  const cost = overhead(load('blocking.jsonl'), load('base.jsonl'))
  assert.deepEqual(cost.currentModes, ['blocking'])
  assert.deepEqual(
    cost.baselineModes,
    ['off'],
    'a baseline from before the verifier was made with the check off',
  )
  assert.equal(cost.medianDuration.now, 11000)
  assert.equal(cost.medianDuration.before, 10000)
  assert.ok(Math.abs(cost.medianDuration.percent - 10) < 1e-9)
  assert.equal(cost.medianSteps.delta, 1)
  const { markdown, verdict } = render(load('blocking.jsonl'), load('base.jsonl'))
  assert.ok(
    markdown.indexOf('## Gate:') < markdown.indexOf('## Verifier overhead'),
    'the gate stays first',
  )
  assert.match(markdown, /Check modes: this run `blocking`, baseline `off`/)
  assert.match(markdown, /\| median duration \| 11\.0s \| 10\.0s \| \+10\.0% \|/)
  assert.match(markdown, /\| median steps \| 5 \| 4 \| \+1 \|/)
  assert.equal(verdict.verdict, 'PASS', 'exactly 10% slower is inside the gate, as before')
})