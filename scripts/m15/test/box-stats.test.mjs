import assert from 'node:assert/strict'
import { test } from 'node:test'

import { boxReport, boxStats, isBoxMode, median } from '../box-stats.mjs'

const row = (mode, size, iou, extra = {}) => ({
  mode,
  size,
  target: `t-${String(iou)}`,
  scene: 's',
  ok: iou !== undefined,
  iou,
  iou50: iou === undefined ? undefined : iou >= 0.5,
  centre_inside: iou === undefined ? undefined : iou > 0.1,
  ...extra,
})

test('a mode label is a box mode only when it ends in :box', () => {
  for (const mode of ['gemma:box', 'gemma-4-12b-it:box@1280', 'x:box+nothink', 'x:box@640+nothink'])
    assert.ok(isBoxMode(mode), mode)
  for (const mode of ['gemma:norm', 'gemma:boxes', 'box:norm', 'gemma:ground', 'gemma:px'])
    assert.ok(!isBoxMode(mode), mode)
})

test('the median of odd and even counts', () => {
  assert.equal(median([0.9, 0.1, 0.5]), 0.5)
  assert.equal(median([0.2, 0.4, 0.6, 0.8]), 0.5)
  assert.equal(median([]), null)
})

test('box numbers by mode and size, worked by hand', () => {
  const rows = [
    // large: IoU 0.9, 0.7 (both >= 0.5), 0.5 (exactly the bar)
    row('m:box', 'large', 0.9),
    row('m:box', 'large', 0.7),
    row('m:box', 'large', 0.5),
    // medium: 0.6, 0.2
    row('m:box', 'medium', 0.6),
    row('m:box', 'medium', 0.2),
    // small: 0.4, and one answer that never came
    row('m:box', 'small', 0.4),
    row('m:box', 'small', undefined),
    // another mode and a non-box mode in the same file
    row('n:box', 'large', 1),
    { mode: 'm:norm', size: 'large', hit: true, ok: true },
  ]
  const stats = boxStats(rows)
  assert.deepEqual(
    stats.map((entry) => entry.mode),
    ['m:box', 'n:box'],
    'only box modes, in order of appearance',
  )
  const [m, n] = stats
  assert.equal(m.overall.asked, 7)
  assert.equal(m.overall.answered, 6)
  assert.equal(m.overall.iou50, 4, '0.9, 0.7, 0.5 and 0.6')
  assert.ok(
    Math.abs(m.overall.meanIou - (0.9 + 0.7 + 0.5 + 0.6 + 0.2 + 0.4) / 6) < 1e-12,
    'the mean is over answered boxes',
  )
  assert.equal(
    m.overall.medianIou,
    0.55,
    'the middle two of 0.2 0.4 0.5 0.6 0.7 0.9 are 0.5 and 0.6',
  )
  assert.deepEqual(
    [m.bySize.large.asked, m.bySize.large.iou50, m.bySize.large.medianIou],
    [3, 3, 0.7],
  )
  assert.deepEqual(
    [m.bySize.medium.asked, m.bySize.medium.iou50, m.bySize.medium.medianIou],
    [2, 1, 0.4],
  )
  assert.deepEqual(
    [m.bySize.small.asked, m.bySize.small.answered, m.bySize.small.iou50, m.bySize.small.medianIou],
    [2, 1, 0, 0.4],
    'an unanswered request counts as asked and as a miss',
  )
  assert.equal(n.overall.asked, 1)
  assert.equal(n.overall.iou50, 1)
})

test('with no box runs there is nothing to report and the older conventions are left alone', () => {
  assert.deepEqual(boxReport([{ mode: 'm:norm', size: 'small', hit: false, ok: true }]), [])
  assert.deepEqual(boxStats([]), [])
})

test('the report text carries the table and the size table', () => {
  const text = boxReport([
    row('m:box', 'large', 0.8),
    row('m:box', 'small', 0.3),
    row('m:box', 'small', undefined),
  ]).join('\n')
  assert.match(text, /## Box accuracy \(IoU\)/)
  assert.match(text, /\| m:box \| 3 \| 67% \| 0\.55 \| 0\.55 \| 33% \(1\/3\) \|/)
  assert.match(
    text,
    /\| m:box \| 0% \(0\/2\), 0\.30 \| - \| 100% \(1\/1\), 0\.80 \|/,
    'small, medium (none), large',
  )
})

test('the app report prompt modes are box modes too, and rejected boxes are counted', () => {
  assert.ok(isBoxMode('m:analysis'))
  assert.ok(isBoxMode('m:analysis-point@1280+nothink'))
  assert.ok(!isBoxMode('m:analysisx'))
  const rows = [
    {
      mode: 'm:analysis',
      size: 'small',
      iou: 0.6,
      iou50: true,
      centre_inside: true,
      boxes_rejected: 1,
    },
    {
      mode: 'm:analysis',
      size: 'large',
      iou: 0.1,
      iou50: false,
      centre_inside: false,
      boxes_rejected: 0,
    },
  ]
  const [stat] = boxStats(rows)
  assert.equal(stat.overall.rejected, 1)
  assert.ok(boxReport(rows).some((line) => line.includes('quality guard')))
  assert.ok(
    !boxReport([{ mode: 'm:box', size: 'small', iou: 1, iou50: true, centre_inside: true }]).some(
      (line) => line.includes('quality guard'),
    ),
  )
})