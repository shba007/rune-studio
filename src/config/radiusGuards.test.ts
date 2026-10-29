import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vite-plus/test'

const dir = resolve(process.cwd(), 'src/components')
const ALLOWED = /^rounded(?:-[trbl]{1,2})?-(?:full|none|rune-tile|rune-panel|rune-control)$/
const ANY = /(?<![\w-])rounded[\w[\].-]*/g

const LEGACY: Record<string, number> = {}

const counts = readdirSync(dir)
  .filter((name) => name.endsWith('.vue'))
  .map((name) => {
    const source = readFileSync(resolve(dir, name), 'utf8')
    const offenders = (source.match(ANY) ?? []).filter((cls) => !ALLOWED.test(cls))
    return [name, offenders.length] as const
  })
  .filter(([, count]) => count > 0)

it('adds no radius outside circle, pill and the tile token', () => {
  const grown = counts
    .filter(([name, count]) => count > (LEGACY[name] ?? 0))
    .map(([name, count]) => `${name}:${count}`)
  expect(grown).toEqual([])
})