import { describe, expect, it } from 'vite-plus/test'
import { outcomeProgress, planProgress, taskProgress } from './workspaceProgress'
import type { WorkspaceTask, OutcomeMetric } from '@/types/workspace'
const task = (states: string[]): WorkspaceTask => ({
  id: 'a',
  revision: 1,
  title: 'Walk',
  parentGoalId: null,
  class: 'general',
  createdMs: 0,
  schedule: null,
  paused: false,
  error: null,
  completedSteps: 1,
  totalSteps: 4,
  occurrences: states.map((state, i) => ({
    id: String(i),
    state,
    workId: 'same-work',
    dueMs: 1,
    runId: null,
    reported: false,
    completedMs: null,
  })),
})
describe('evidence-based progress', () => {
  it('measures increasing and decreasing outcomes from the stated baseline', () => {
    const metric: OutcomeMetric = {
      baseline: 100,
      current: 90,
      target: 80,
      unit: 'kg',
      note: '',
      updatedAt: '',
    }
    expect(outcomeProgress(metric)).toBe(50)
    expect(outcomeProgress({ ...metric, baseline: 0, current: 25, target: 100 })).toBe(25)
    expect(outcomeProgress(null)).toBeNull()
    expect(outcomeProgress({ ...metric, current: NaN })).toBeNull()
  })
  it('counts retried work once and excludes replaced schedules', () => {
    expect(planProgress([task(['failed', 'completed'])])).toEqual({ done: 1, total: 1, value: 100 })
    expect(planProgress([task(['rescheduled'])]).value).toBeNull()
    expect(planProgress([task(['pending'])]).value).toBe(0)
  })
  it('does not turn elapsed time or manual activity into completed progress', () => {
    expect(taskProgress(task(['running']))).toBe(25)
    expect(taskProgress({ ...task(['in_progress']), executionMode: 'manual' })).toBe(0)
    expect(taskProgress({ ...task(['pending']), totalSteps: 0 })).toBeNull()
  })
})