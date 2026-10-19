// Box accuracy for the `box` convention of the M1.5 probe: how well the boxes a model draws
// match the real element, by IoU. Used by report.mjs; kept apart so it can be tested.

/** Whether a mode label is a `box` run (`model:box`, optionally with `@edge` and `+nothink`). */
export const isBoxMode = (mode) => /:box(@\d+)?(\+nothink)?$/.test(mode)

const sorted = (values) => [...values].sort((a, b) => a - b)

/** The middle value (the mean of the two middle ones for an even count); null for none. */
export function median(values) {
  if (values.length === 0) return null
  const list = sorted(values)
  const middle = Math.floor(list.length / 2)
  return list.length % 2 === 1 ? list[middle] : (list[middle - 1] + list[middle]) / 2
}

function mean(values) {
  return values.length === 0
    ? null
    : values.reduce((total, value) => total + value, 0) / values.length
}

/** One group of box answers. `asked` counts every request, so an answer that never came is a miss. */
function summarize(rows) {
  const answered = rows.filter((row) => typeof row.iou === 'number')
  return {
    asked: rows.length,
    answered: answered.length,
    meanIou: mean(answered.map((row) => row.iou)),
    medianIou: median(answered.map((row) => row.iou)),
    iou50: rows.filter((row) => row.iou50 === true).length,
    centreInside: rows.filter((row) => row.centre_inside === true).length,
  }
}

/**
 * Per box mode: the overall numbers and the same by size class (small, medium, large).
 * Returns `[{ mode, overall, bySize: { small, medium, large } }]` in the order the modes appear.
 */
export function boxStats(rows) {
  const modes = []
  for (const row of rows) if (isBoxMode(row.mode) && !modes.includes(row.mode)) modes.push(row.mode)
  return modes.map((mode) => {
    const list = rows.filter((row) => row.mode === mode)
    return {
      mode,
      overall: summarize(list),
      bySize: Object.fromEntries(
        ['small', 'medium', 'large'].map((size) => [
          size,
          summarize(list.filter((row) => row.size === size)),
        ]),
      ),
    }
  })
}

const pct = (count, of) => (of === 0 ? '-' : `${((100 * count) / of).toFixed(0)}%`)
const iouText = (value) => (value === null ? '-' : value.toFixed(2))

/** The report text for the box runs (empty when there are none). */
export function boxReport(rows) {
  const stats = boxStats(rows)
  if (stats.length === 0) return []
  const out = ['', '## Box accuracy (IoU)', '']
  out.push(
    'IoU is the overlap of the answered box with the real element, 1 for the same box and 0 for none. `IoU >= 0.5` is the usual bar for "found it". `Centre inside` says whether a click in the middle of the answered box would land on the element. A request with no usable box counts as a miss.',
    '',
  )
  out.push(
    '| Mode | Asked | Answered | Mean IoU | Median IoU | IoU >= 0.5 | Centre inside |',
    '| --- | --- | --- | --- | --- | --- | --- |',
  )
  for (const { mode, overall } of stats) {
    out.push(
      `| ${mode} | ${String(overall.asked)} | ${pct(overall.answered, overall.asked)} | ${iouText(overall.meanIou)} | ${iouText(overall.medianIou)} | ${pct(overall.iou50, overall.asked)} (${String(overall.iou50)}/${String(overall.asked)}) | ${pct(overall.centreInside, overall.asked)} |`,
    )
  }
  out.push(
    '',
    '### By target size',
    '',
    'Small is under 24 px on its short side, medium under 44 px, large the rest. Each cell: share with IoU >= 0.5 (count), median IoU.',
    '',
  )
  out.push('| Mode | Small | Medium | Large |', '| --- | --- | --- | --- |')
  for (const { mode, bySize } of stats) {
    const cell = (group) =>
      group.asked === 0
        ? '-'
        : `${pct(group.iou50, group.asked)} (${String(group.iou50)}/${String(group.asked)}), ${iouText(group.medianIou)}`
    out.push(`| ${mode} | ${cell(bySize.small)} | ${cell(bySize.medium)} | ${cell(bySize.large)} |`)
  }
  return out
}