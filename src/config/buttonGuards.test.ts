import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vite-plus/test'

const dir = resolve(process.cwd(), 'src/components')
const ATOMS =
  /^(?:Rune(Button|IconButton|Segment|NavItem|Chip|Dot|Switch|Choice|Menu)|WorkRow).vue$/

const RAW_BUTTONS: Record<string, number> = {
  'AgentPill.vue': 2,
  'AnalysisFindingRow.vue': 1,
  'ArtifactCard.vue': 1,
  'CaptureTargetPicker.vue': 2,
  'Composer.vue': 1,
  'FlightDrawer.vue': 1,
  'FlightTaskTimeline.vue': 2,
  'GoalsTable.vue': 2,
  'WorkspaceTaskTimeline.vue': 1,
  'KnowledgeOverview.vue': 1,
  'OrbDock.vue': 2,
  'SettingsConnectionModels.vue': 1,
  'TaskBoard.vue': 1,
  'TaskPreview.vue': 1,
  'TitleBar.vue': 3,
}

it('adds no raw button outside the Rune atoms', () => {
  const grown = readdirSync(dir)
    .filter((name) => name.endsWith('.vue') && !ATOMS.test(name))
    .map((name) => {
      const source = readFileSync(resolve(dir, name), 'utf8').split('<style')[0]!
      return [name, (source.match(/<button\b/g) ?? []).length] as const
    })
    .filter(([name, count]) => count > (RAW_BUTTONS[name] ?? 0))
    .map(([name, count]) => name + ':' + count)
  expect(grown).toEqual([])
})