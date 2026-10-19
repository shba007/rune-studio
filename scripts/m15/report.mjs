// Summarises the M1.5 probe results.
//
//   node scripts/m15/report.mjs temp/m15/results.jsonl

import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

import { boxReport } from './box-stats.mjs'

const file = process.argv[2] ?? 'temp/m15/results.jsonl'
const rows = readFileSync(file, 'utf8')
  .split('\n')
  .filter(Boolean)
  .map((line) => JSON.parse(line))

const pct = (n, d) => (d ? `${((100 * n) / d).toFixed(0)}%` : '-')
const sorted = (values) => [...values].sort((a, b) => a - b)
const quantile = (values, q) =>
  values.length ? sorted(values)[Math.min(values.length - 1, Math.floor(q * values.length))] : null
const fmtMs = (ms) =>
  ms == null
    ? '-'
    : ms >= 10_000
      ? `${(ms / 1000).toFixed(0)} s`
      : ms >= 1000
        ? `${(ms / 1000).toFixed(1)} s`
        : `${ms} ms`

const byMode = Map.groupBy(rows, (r) => r.mode)
const lines = []
const out = (text = '') => {
  lines.push(text)
  console.log(text)
}

out(`# M1.5 grounding probe results\n`)
out(
  `${rows.length} requests, ${new Set(rows.map((r) => r.scene)).size} scenes, ${new Set(rows.map((r) => `${r.scene}/${r.target}`)).size} distinct targets.\n`,
)
out(
  '| Mode | Asked | Answered | Hit | Hit or near (12 px) | Median latency | p90 latency | Median miss distance |',
)
out('| --- | --- | --- | --- | --- | --- | --- | --- |')
for (const [mode, list] of byMode) {
  const answered = list.filter((r) => r.ok)
  const hits = answered.filter((r) => r.hit)
  const near = answered.filter((r) => r.hit || r.near)
  const misses = answered.filter((r) => !r.hit)
  const lat = list.map((r) => r.latency_ms)
  out(
    `| ${mode} | ${list.length} | ${pct(answered.length, list.length)} | ${pct(hits.length, list.length)} | ${pct(near.length, list.length)} | ${fmtMs(quantile(lat, 0.5))} | ${fmtMs(quantile(lat, 0.9))} | ${
      misses.length
        ? `${quantile(
            misses.map((r) => r.dist_px),
            0.5,
          )} px`
        : '-'
    } |`,
  )
}

{
  // Modes may have been run on different numbers of targets. Compare like for like.
  const keysOf = (list) => new Set(list.map((r) => `${r.scene}/${r.target}`))
  let common = null
  for (const list of byMode.values()) {
    const k = keysOf(list)
    common = common ? new Set([...common].filter((x) => k.has(x))) : k
  }
  if (common && common.size && [...byMode.values()].some((l) => keysOf(l).size !== common.size)) {
    out(`
## Like for like: the ${common.size} targets every mode was asked
`)
    out('| Mode | Hit | Hit or near | Median latency | Median completion tokens |')
    out('| --- | --- | --- | --- | --- |')
    for (const [mode, list] of byMode) {
      const g = list.filter((r) => common.has(`${r.scene}/${r.target}`))
      out(
        `| ${mode} | ${pct(g.filter((r) => r.hit).length, g.length)} (${g.filter((r) => r.hit).length}/${g.length}) | ${pct(g.filter((r) => r.hit || r.near).length, g.length)} | ${fmtMs(
          quantile(
            g.map((r) => r.latency_ms),
            0.5,
          ),
        )} | ${quantile(
          g.map((r) => r.completion_tokens ?? 0),
          0.5,
        )} |`,
      )
    }
  }
}
out('\n## Hit rate by target size\n')
out(
  'Small is under 24 px on its short side (icon buttons, window controls), medium under 44 px, large the rest.\n',
)
out('| Mode | Small | Medium | Large |')
out('| --- | --- | --- | --- |')
for (const [mode, list] of byMode) {
  const cell = (size) => {
    const group = list.filter((r) => r.size === size)
    return group.length
      ? `${pct(group.filter((r) => r.hit).length, group.length)} (${group.filter((r) => r.hit).length}/${group.length})`
      : '-'
  }
  out(`| ${mode} | ${cell('small')} | ${cell('medium')} | ${cell('large')} |`)
}

out('\n## Hit rate by kind of element\n')
const roles = [...new Set(rows.map((r) => r.role))].sort((a, b) => a.localeCompare(b))
out(`| Role | ${[...byMode.keys()].join(' | ')} |`)
out(`| --- | ${[...byMode.keys()].map(() => '---').join(' | ')} |`)
for (const role of roles) {
  out(
    `| ${role} | ${[...byMode.values()]
      .map((list) => {
        const g = list.filter((r) => r.role === role)
        return g.length ? `${pct(g.filter((r) => r.hit).length, g.length)} (${g.length})` : '-'
      })
      .join(' | ')} |`,
  )
}

const problems = rows.filter((r) => !r.ok)
if (problems.length) {
  out('\n## Requests with no usable answer\n')
  for (const r of problems.slice(0, 12)) out(`- ${r.mode} · ${r.scene} · "${r.target}": ${r.note}`)
}

const worst = rows
  .filter((r) => r.ok && !r.hit && r.dist_px != null)
  .sort((a, b) => b.dist_px - a.dist_px)
  .slice(0, 8)
if (worst.length) {
  out('\n## Largest misses\n')
  for (const r of worst)
    out(`- ${r.mode} · ${r.scene} · "${r.target}" (${r.size}): ${r.dist_px} px from the centre`)
}

const reasoning = rows.filter((r) => r.reasoning_chars != null && r.completion_tokens)
if (reasoning.length) {
  out(
    `\nMedian completion: ${quantile(
      reasoning.map((r) => r.completion_tokens),
      0.5,
    )} tokens, of which reasoning text is about ${quantile(
      reasoning.map((r) => r.reasoning_chars),
      0.5,
    )} characters.`,
  )
}

for (const line of boxReport(rows)) out(line)

writeFileSync(join(dirname(file), 'summary.md'), lines.join('\n'))
console.log(`\n(saved ${join(dirname(file), 'summary.md')})`)