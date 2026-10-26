import { expect, it } from 'vite-plus/test'
import { taskTimeline } from './taskTimeline'

import { historyFixture } from '@/components/taskTimelineTestKit'

it('preserves three parallel task lanes and their independent outcomes on a shared request clock', () => {
  const timeline = taskTimeline(historyFixture)!
  expect(timeline.total).toBe(30000)
  expect(timeline.rows).toHaveLength(3)
  expect(timeline.rows.map((row) => row.phase)).toEqual(['done', 'failed', 'cancelled'])
  expect(timeline.rows[0]!.start).toBe(timeline.rows[1]!.start)
  expect(timeline.rows[2]!.steps[0]!.start).toBe(10000)
  expect(timeline.rows[0]!.preparation).toContainEqual({ title: 'Planning', start: 0, end: 1000 })
  expect(timeline.rows[2]!.preparation).toContainEqual({
    title: 'Task active',
    start: 5000,
    end: 10000,
  })
})

it('orders revisions and never invents execution for a task cancelled while queued', () => {
  const snapshot = {
    request: {},
    tasks: [
      { id: 1, text: 'Queued', phase: 'cancelled', startedAt: null, finishedAt: 10000, steps: [] },
    ],
  }
  const timeline = taskTimeline([
    { revision: 3, atMs: 10000, snapshot },
    { revision: 1, atMs: 1000, snapshot: { tasks: [] } },
  ])!
  expect(timeline.rows[0]!.start).toBe(timeline.rows[0]!.end)
  expect(taskTimeline([])).toBeNull()
  expect(taskTimeline([{ revision: 'bad', atMs: 1000 }])).toBeNull()
})