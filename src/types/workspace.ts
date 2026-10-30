import type { IntakeEntry } from './decisions'
export interface TaskSchedule {
  local: string
  zone: string
  repeat: 'once' | 'daily' | 'weekly' | 'monthly'
  nextLocal: string
  nextMs: number
  anchorDay?: number
}
export interface TaskOccurrence {
  id: string
  dueMs: number | null
  state: string
  runId: string | null
  reported: boolean
  completedMs: number | null
  workId?: string | null
  snoozedUntilMs?: number | null
}
export interface WorkspaceTask {
  executionMode?: 'manual' | 'agent' | 'analysis'
  dueAction?: 'remind' | 'run'
  id: string
  revision: number
  title: string
  parentGoalId: string | null
  class: IntakeEntry['class']
  createdMs: number
  schedule: TaskSchedule | null
  paused: boolean
  occurrences: TaskOccurrence[]
  error: string | null
  completedSteps: number
  totalSteps: number
  legacyTaskId?: string | null
  instructions?: string | null
}
export interface OutcomeMetric {
  baseline: number
  current: number
  target: number
  unit: string
  note: string
  updatedAt: string
}
export interface CalendarItem {
  taskId: string
  atMs: number
  forecast: boolean
  state: string
}