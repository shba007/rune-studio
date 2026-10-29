import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vite-plus/test'

const dir = resolve(process.cwd(), 'src/components')
const components = readdirSync(dir)
  .filter((name) => name.endsWith('.vue'))
  .map((name) => ({
    name,
    source: readFileSync(resolve(dir, name), 'utf8').split('<style')[0]!,
  }))

const offenders = (pattern: RegExp) =>
  components.filter(({ source }) => pattern.test(source)).map(({ name }) => name)

it('uses design tokens instead of arbitrary hex colour classes', () => {
  expect(offenders(/-\[#[0-9a-fA-F]{3,8}\]/)).toEqual([])
})

it('uses the five rune type roles, not the retired scale', () => {
  expect(offenders(/(?<![\w-])text-(?:title|body|status|label|caption)(?![\w-])/)).toEqual([])
})

it('leaves no palette colour classes except the warning amber', () => {
  expect(offenders(/(?<![\w-])(?:text|bg)-(?:zinc|slate|gray|neutral|stone)-\d+/)).toEqual([])
})
it('uses only Oxanium and Figtree as font families', () => {
  const css = readFileSync(resolve(process.cwd(), 'src/style.css'), 'utf8')
  const families = [...css.matchAll(/^\s*--font-(?!weight)[\w-]+:\s*([^;]+);/gm)].map((m) => m[1]!)
  expect(families.length).toBeGreaterThan(0)
  for (const family of families) {
    expect(family).toMatch(/^'(?:Oxanium|Figtree) Variable', sans-serif$/)
  }
  const stray = components.filter(({ source }) => /font-(?:serif)|font-\[/.test(source))
  expect(stray.map(({ name }) => name)).toEqual([])
})