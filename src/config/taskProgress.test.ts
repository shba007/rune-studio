import { expect, it } from 'vite-plus/test'
import type { IndependentTask, PlannedRequest } from '@/composables/useTaskBoard'
import {
  currentStep,
  estimateRemaining,
  requestProgress,
  taskElapsed,
  taskPosition,
  visibleGoal,
} from './taskProgress'
const task = (over: Partial<IndependentTask> = {}): IndependentTask => ({
  id: 1,
  requestId: 1,
  goalId: 'g1',
  text: 'Finite task',
  kind: 'background',
  dependsOn: [],
  phase: 'running',
  currentStep: 2,
  startedAt: 1000,
  stepStartedAt: 7000,
  finishedAt: null,
  steps: [
    { text: 'Read', phase: 'done' },
    { text: 'Summarize', phase: 'running' },
  ],
  result: null,
  ...over,
})
it('counts completed steps across parallel tasks and keeps unfinished work partial', () => {
  expect(requestProgress([])).toBeNull()
  expect(requestProgress([task()])).toBe(0.5)
  expect(requestProgress([task(), task({ id: 2, phase: 'queued', steps: [] })])).toBe(1 / 3)
  expect(requestProgress([task({ phase: 'done' })])).toBe(1)
  expect(requestProgress([task({ phase: 'failed' })])).toBe(0.5)
})
it('keeps queued and terminal task elapsed times stable and counts every planned step', () => {
  expect(taskElapsed(task({ startedAt: null, phase: 'queued', currentStep: 0 }), 9000)).toBe(0)
  expect(taskElapsed(task(), 9000)).toBe(8000)
  const done = task({ phase: 'done', finishedAt: 8000 })
  expect(taskElapsed(done, 9000)).toBe(7000)
  expect(taskElapsed(done, 90000)).toBe(7000)
  expect(currentStep(done)).toBe(2)
  expect(taskPosition([task(), task({ id: 2, phase: 'queued', currentStep: 0 })])).toBe('1/2')
})
it('estimates from measured completed steps and does not invent an initial or overdue ETA', () => {
  expect(
    estimateRemaining(
      [task({ currentStep: 1, stepStartedAt: 1000, steps: [{ text: 'Read', phase: 'running' }] })],
      2000,
    ),
  ).toBeNull()
  expect(estimateRemaining([task()], 9000)).toBe(4000)
  expect(estimateRemaining([task()], 15000)).toBeNull()
  expect(estimateRemaining([task({ phase: 'done', finishedAt: 8000 })], 9000)).toBe(0)
})
it('shows one current goal and moves to the next queued goal only after the first is terminal', () => {
  const goal = (id: number, phase: PlannedRequest['phase']): PlannedRequest => ({
    id,
    phase,
    text: `Prompt ${id}`,
    goals: [],
    dependsOn: [],
    error: null,
  })
  expect(visibleGoal([goal(1, 'running'), goal(2, 'queued')])?.id).toBe(1)
  expect(visibleGoal([goal(1, 'done'), goal(2, 'queued')])?.id).toBe(2)
})