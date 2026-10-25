import { flushPromises, mount } from '@vue/test-utils'
import { nextTick, ref } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import type { IndependentTask, TaskBoardModel } from '@/composables/useTaskBoard'
import TaskBoard from './TaskBoard.vue'
const NOW = 100000
function fixture(): TaskBoardModel {
  const base = Date.now()
  const task = (id: number, phase: IndependentTask['phase']): IndependentTask => ({
    id,
    requestId: 1,
    goalId: 'g1',
    kind: 'background',
    text: `Objective ${id}`,
    phase,
    dependsOn: [],
    currentStep: phase === 'queued' ? 0 : 2,
    startedAt: phase === 'queued' ? null : base - 10000,
    stepStartedAt: phase === 'queued' ? null : base - 1000,
    finishedAt: phase === 'done' ? base - 3000 : null,
    steps: [
      {
        text: 'Read input',
        phase: phase === 'queued' ? 'queued' : 'done',
      },
      {
        text: 'Produce answer',
        phase: phase === 'done' ? 'done' : phase === 'running' ? 'running' : 'queued',
      },
    ],
    result: phase === 'done' ? 'Saved answer' : null,
  })
  return {
    tasks: ref([task(1, 'running'), task(2, 'done'), task(3, 'queued')]),
    requests: ref([
      {
        id: 1,
        dependsOn: [],
        text: 'raw original prompt with extra words',
        phase: 'running',
        error: null,
        goals: [
          {
            id: 'g1',
            title: 'Summarize the supplied input and check its facts',
            dependsOn: [],
            tasks: [1, 2, 3],
          },
        ],
      },
      {
        id: 2,
        dependsOn: [],
        text: 'Do not show this queued goal yet',
        phase: 'queued',
        error: null,
        goals: [],
      },
    ]),
    notice: ref(''),
    pending: ref(false),
    submit: vi.fn<(text: string) => Promise<boolean>>(async () => true),
    cancel: vi.fn<(id: number) => Promise<void>>(async () => {}),
    cancelRequest: vi.fn<(id: number) => Promise<void>>(async () => {}),
    dismiss: vi.fn<(id: number) => Promise<void>>(async () => {}),
  }
}
afterEach(() => vi.useRealTimers())
it('shows one cleaned goal once, counters on every task, and parallel carousel controls', async () => {
  vi.useFakeTimers()
  vi.setSystemTime(NOW)
  const board = fixture()
  const wrapper = mount(TaskBoard, { props: { board } })
  await nextTick()
  expect(wrapper.get('h2').text()).toBe('Tasks Board')
  expect(wrapper.get('[data-testid="goal-title"]').text()).toBe(
    'Summarize the supplied input and check its facts',
  )
  expect(wrapper.text()).not.toContain('raw original prompt')
  expect(wrapper.text()).not.toContain('Do not show this queued goal yet')
  expect(wrapper.text()).toContain('1 goal queued')
  expect(wrapper.get('[data-testid="task-count"]').text()).toBe('Tasks 1/3')
  expect(wrapper.get('[data-testid="goal-eta"]').text()).toContain('left')
  expect(wrapper.findAll('[data-testid="task-elapsed"]')).toHaveLength(3)
  expect(wrapper.get('[data-task-id="1"] [data-testid="step-count"]').text()).toBe('Step 2/2')
  expect(wrapper.get('[data-task-id="2"] [data-testid="task-elapsed"]').text()).toBe('0:07')
  expect(wrapper.get('[data-task-id="3"] [data-testid="task-elapsed"]').text()).toBe('0:00')
  expect(wrapper.get('[data-testid="task-carousel"]').classes()).toContain('parallel-tasks')
  expect(wrapper.find('[aria-label="Next task"]').exists()).toBe(true)
  await vi.advanceTimersByTimeAsync(2000)
  await nextTick()
  expect(wrapper.get('[data-task-id="1"] [data-testid="task-elapsed"]').text()).toBe('0:12')
  expect(wrapper.get('[data-task-id="2"] [data-testid="task-elapsed"]').text()).toBe('0:07')
  wrapper.unmount()
})
it('defaults to active-only expansion, preserves manual expansion, and scrolls to a newly active task', async () => {
  const board = fixture()
  const wrapper = mount(TaskBoard, { props: { board } })
  await flushPromises()
  expect(wrapper.get('[data-task-id="1"] [data-testid="task-detail"]').isVisible()).toBe(true)
  expect(wrapper.find('[data-task-id="2"] [data-testid="task-detail"]').exists()).toBe(false)
  expect(wrapper.find('[data-task-id="3"] [data-testid="task-detail"]').exists()).toBe(false)
  await wrapper.get('[aria-label="Expand task 2"]').trigger('click')
  expect(wrapper.get('[data-task-id="2"] [data-testid="task-detail"]').isVisible()).toBe(true)
  const scroll = vi.fn<(options?: ScrollToOptions) => void>()
  Object.defineProperty(wrapper.get('[data-testid="task-carousel"]').element, 'scrollTo', {
    value: scroll,
  })
  Object.defineProperty(wrapper.get('[data-task-id="3"]').element, 'offsetLeft', { value: 500 })
  board.tasks.value[0]!.phase = 'done'
  board.tasks.value[0]!.finishedAt = Date.now()
  board.tasks.value[2]!.phase = 'running'
  board.tasks.value[2]!.startedAt = Date.now()
  board.tasks.value[2]!.currentStep = 1
  await flushPromises()
  expect(scroll).toHaveBeenCalledWith(expect.objectContaining({ left: 500, top: 0 }))
  expect(wrapper.find('[data-task-id="1"] [data-testid="task-detail"]').exists()).toBe(false)
  expect(wrapper.get('[data-task-id="2"] [data-testid="task-detail"]').isVisible()).toBe(true)
  expect(wrapper.get('[data-task-id="3"] [data-testid="task-detail"]').isVisible()).toBe(true)
  await wrapper.get('[aria-label="Cancel task 3"]').trigger('click')
  expect(board.cancel).toHaveBeenCalledExactlyOnceWith(3)
  wrapper.unmount()
})