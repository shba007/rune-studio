import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vite-plus/test'

const dir = resolve(process.cwd(), 'src/components')
const components = readdirSync(dir)
  .filter((name) => name.endsWith('.vue'))
  .map((name) => ({ name, source: readFileSync(resolve(dir, name), 'utf8') }))

const OWN_FOCUS_STYLE = new Set(['KnowledgeOverview.vue', 'SettingsLife.vue'])

it('leaves keyboard focus to the single global ring', () => {
  const found = components
    .filter(({ name }) => !OWN_FOCUS_STYLE.has(name))
    .filter(({ source }) => /focus(?:-visible)?:ring-/.test(source))
    .map(({ name }) => name)
  expect(found).toEqual([])
})

it('pulses only when motion is allowed', () => {
  const found = components
    .filter(({ source }) => /(?<!motion-safe:)animate-pulse/.test(source))
    .map(({ name }) => name)
  expect(found).toEqual([])
})