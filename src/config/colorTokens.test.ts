import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vite-plus/test'

const PILL_PATH = [
  'AgentPill',
  'Composer',
  'OrbDock',
  'ConfirmCard',
  'TaskBoard',
  'TaskPanel',
  'TaskReminder',
  'DecisionInbox',
  'AnalysisCard',
  'ReflectionCard',
  'FeedbackCard',
  'ReviewCard',
]
const ALLOWED = new Set(['#a1a1aa', '#52525b'])
const source = (name: string) =>
  readFileSync(resolve(process.cwd(), 'src/components', `${name}.vue`), 'utf8')

it('keeps hard-coded colours out of the pill-path components', () => {
  const found = PILL_PATH.flatMap((name) =>
    (source(name).match(/#[0-9a-fA-F]{6}\b/g) ?? [])
      .filter((hex) => !ALLOWED.has(hex.toLowerCase()))
      .map((hex) => `${name}: ${hex}`),
  )
  expect(found).toEqual([])
})

it('keeps one global focus ring in the base layer', () => {
  const css = readFileSync(resolve(process.cwd(), 'src/style.css'), 'utf8')
  expect(css).toMatch(/@layer base\s*{\s*:focus-visible\s*{[^}]*outline:[^;]*var\(--accent\)/)
  for (const name of ['AgentPill', 'TaskBoard', 'FeedbackCard'])
    expect(source(name)).not.toMatch(/focus-visible:ring/)
})

it('uses the error token instead of the raw error red anywhere', () => {
  const names = [
    ...PILL_PATH,
    'SettingsLife',
    'SettingsConnectionModels',
    'FlightDrawer',
    'WorkspaceTaskTimeline',
    'FlightTimeline',
    'FlightTaskTimeline',
  ]
  expect(names.filter((name) => source(name).includes('#ff6b72'))).toEqual([])
})