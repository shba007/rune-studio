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