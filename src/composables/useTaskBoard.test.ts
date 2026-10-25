import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import { useTaskBoard, type IndependentTask, type TaskBoardModel } from './useTaskBoard'

type Snapshot = { revision: number; tasks: IndependentTask[] }
const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
  event: null as null | ((event: { payload: Snapshot }) => void),
  off: vi.fn<() => void>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<
    (name: string, callback: (event: { payload: Snapshot }) => void) => Promise<() => void>
  >(async (_, callback) => {
    harness.event = callback
    return harness.off
  }),
}))
afterEach(() => vi.clearAllMocks())
it('keeps the newest per-task state and routes submission and cancellation by ID', async () => {
  let initial: ((value: Snapshot) => void) | undefined
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'tasks_state')
      return new Promise((resolve) => {
        initial = resolve
      })
    return 42
  })
  let board: TaskBoardModel | undefined
  const wrapper = mount(
    defineComponent({
      setup() {
        board = useTaskBoard()
        return () => null
      },
    }),
  )
  await flushPromises()
  const running: IndependentTask = {
    id: 42,
    requestId: 1,
    goalId: 'g1',
    dependsOn: [],
    steps: [{ text: 'Summarize', phase: 'running' }],
    kind: 'background',
    phase: 'running',
    text: 'Independent task',
    result: null,
  }
  harness.event!({ payload: { revision: 2, tasks: [running] } })
  initial!({ revision: 1, tasks: [] })
  await flushPromises()
  expect(board!.tasks.value).toEqual([running])
  harness.event!({ payload: { revision: 3, tasks: [{ ...running, phase: 'cancelled' }] } })
  harness.event!({ payload: { revision: 2, tasks: [running] } })
  expect(board!.tasks.value[0]!.phase).toBe('cancelled')
  await board!.submit('Another task')
  expect(harness.invoke).toHaveBeenCalledWith('submit_independent_task', { text: 'Another task' })
  await board!.cancel(42)
  expect(harness.invoke).toHaveBeenCalledWith('cancel_independent_task', { id: 42 })
  wrapper.unmount()
  expect(harness.off).toHaveBeenCalledOnce()
})