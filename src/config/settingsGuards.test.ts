import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vite-plus/test'

const dir = resolve(process.cwd(), 'src/components')
const migrated = [
  'SettingsGeneral.vue',
  'SettingsAgents.vue',
  'SettingsShortcuts.vue',
  'HubToggle.vue',
  'HubRow.vue',
  'HubGroup.vue',
  'SettingsConnectionModels.vue',
  'SettingsTools.vue',
  'ModelRoleRow.vue',
  'SettingsReview.vue',
  'SettingsGoalContext.vue',
  'SettingsVisualAnalysis.vue',
  'CaptureTargetPicker.vue',
  'RecordingControls.vue',
  'SettingsAbout.vue',
  'DevInputLab.vue',
  'DecisionInbox.vue',
  'DecisionCard.vue',
  'GoalSummary.vue',
  'GoalWorkPanel.vue',
  'GoalPathways.vue',
  'GoalTaskDraft.vue',
  'GoalTaskProposal.vue',
  'KnowledgeOverview.vue',
  'PersonalitiesPanel.vue',
  'NewWorkForm.vue',
]

it('uses Rune controls, not native selects or checkboxes, on migrated settings pages', () => {
  const offenders = migrated.filter((name) =>
    /<select\b|type="checkbox"/.test(readFileSync(resolve(dir, name), 'utf8')),
  )
  expect(offenders).toEqual([])
})

it('keeps every migrated settings file present', () => {
  const present = new Set(readdirSync(dir))
  expect(migrated.filter((name) => !present.has(name))).toEqual([])
})