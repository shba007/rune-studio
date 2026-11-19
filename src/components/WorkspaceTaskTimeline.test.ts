import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import WorkspaceTaskTimeline from './WorkspaceTaskTimeline.vue'
import TaskPreview from './TaskPreview.vue'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import type { PreviewRun } from '@/config/preview'
import type { WorkspaceTask } from '@/types/workspace'
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

beforeEach(() => invoke.mockReset().mockImplementation(async () => ({})))
const now = Date.now()
const run = (id: string, task: string, outcome: string | null, startedMs: number): PreviewRun => ({
  id,
  task,
  source: 'built-in',
  outcome,
  startedUtc: new Date(startedMs).toISOString(),
  endedUtc: null,
  steps: 1,
  verdict: null,
  thumb: 'frame.jpg',
})

function mountTimeline(history: PreviewRun[], tasks: WorkspaceTask[] = []) {
  const action = vi.fn<(task: WorkspaceTask, name: string) => Promise<void>>()
  const workspace = {
    tasks: ref(tasks),
    error: ref(''),
    pending: ref(false),
    action,
    link: vi.fn<() => Promise<void>>(),
  } as unknown as WorkspaceTasksModel
  const wrapper = mount(WorkspaceTaskTimeline, {
    props: { workspace, history, goals: [], active: true },
    global: { stubs: { TaskPreview: true } },
  })
  return { wrapper, action }
}

it('groups unlinked work under Misc, newest first, and opens the matching replay', async () => {
  localStorage.setItem('rune.tasksView', 'cards')
  const { wrapper } = mountTimeline([
    run('run-b', 'Older job', 'failed', now - 3 * 86_400_000),
    run('run-a', 'Calculate 14 + 27', 'succeeded', now),
  ])
  await flushPromises()
  expect(wrapper.findAll('h3').map((heading) => heading.text())).toEqual(['Misc'])
  expect(
    wrapper.findAll('[data-work-task]').map((card) => card.attributes('data-work-task')),
  ).toEqual(['run-a', 'run-b'])
  const preview = wrapper.getComponent(TaskPreview)
  expect(preview.props('run')).toMatchObject({ id: 'run-a' })
  preview.vm.$emit('open')
  expect(wrapper.emitted('open')).toEqual([['run-a']])
  expect(wrapper.get('[data-work-task="run-a"]').text()).toContain('Calculate 14 + 27')
  wrapper.unmount()
})

it('groups linked tasks under their goal and opens the goal from the heading', async () => {
  const task = {
    id: 'task-1',
    revision: 1,
    title: 'Run-walk',
    parentGoalId: 'goal-1',
    class: 'health',
    createdMs: now,
    schedule: null,
    paused: false,
    executionMode: 'manual',
    occurrences: [],
    error: null,
    completedSteps: 0,
    totalSteps: 0,
  } as unknown as WorkspaceTask
  const action = vi.fn<(task: WorkspaceTask, name: string) => Promise<void>>()
  const workspace = {
    tasks: ref([task]),
    error: ref(''),
    pending: ref(false),
    action,
    link: vi.fn<() => Promise<void>>(),
  } as unknown as WorkspaceTasksModel
  const wrapper = mount(WorkspaceTaskTimeline, {
    props: {
      workspace,
      history: [run('run-a', 'Loose job', 'succeeded', now)],
      goals: [{ id: 'goal-1', class: 'health', approvedGoal: { outcome: 'Get fit' } }] as never,
      active: true,
    },
    global: { stubs: { TaskPreview: true } },
  })
  await flushPromises()
  expect(wrapper.findAll('h3').map((heading) => heading.text())).toEqual(['Get fit', 'Misc'])
  await wrapper
    .findAll('button')
    .find((button) => button.text().startsWith('Open goal'))!
    .trigger('click')
  expect(wrapper.emitted('goal')).toEqual([['goal-1']])
  wrapper.unmount()
})

it('filters rows by status and explains an empty filter', async () => {
  const { wrapper } = mountTimeline([
    run('run-a', 'Finished work', 'succeeded', now),
    run('run-b', 'Broken work', 'failed', now),
  ])
  await flushPromises()
  const pick = (name: string) =>
    wrapper
      .findAll('[role="radio"]')
      .find((button) => button.text() === name)!
      .trigger('click')
  await pick('Failed')
  expect(
    wrapper.findAll('[data-work-task]').map((row) => row.attributes('data-work-task')),
  ).toEqual(['run-b'])
  await pick('Active')
  expect(wrapper.text()).toContain('Nothing matches this filter')
  wrapper.unmount()
})

it('keeps secondary actions in the row menu and routes them', async () => {
  const { wrapper } = mountTimeline([run('run-a', 'Finished work', 'succeeded', now)])
  await flushPromises()
  await wrapper.get('button[aria-haspopup="menu"]').trigger('click')
  const items = wrapper.findAll('[role="menuitem"]').map((item) => item.text())
  expect(items).toEqual(['Review', 'Link goal or schedule', 'Copy prompt', 'Delete'])
  await wrapper
    .findAll('[role="menuitem"]')
    .find((item) => item.text() === 'Delete')!
    .trigger('click')
  expect(wrapper.emitted('remove')).toEqual([['run-a']])
  wrapper.unmount()
})
it('lists tasks as a table with a preview in each row and opens the player from it', async () => {
  localStorage.removeItem('rune.tasksView')
  const { wrapper } = mountTimeline([run('run-a', 'Finished work', 'succeeded', now)])
  await flushPromises()
  expect(wrapper.find('[role="table"]').exists()).toBe(true)
  expect(wrapper.find('[data-testid="peek"]').exists()).toBe(false)
  wrapper.findComponent(TaskPreview).vm.$emit('open')
  expect(wrapper.emitted('open')).toEqual([['run-a']])
  wrapper.unmount()
})
const waitingTask = (mode: 'manual' | 'agent', state = 'pending') =>
  ({
    id: 'task-9',
    revision: 1,
    title: 'Pay the invoice',
    parentGoalId: null,
    class: 'general',
    createdMs: now,
    schedule: null,
    paused: false,
    executionMode: mode,
    occurrences: [{ id: 'o1', dueMs: now + 3_600_000, state, runId: null, reported: false }],
    error: null,
    completedSteps: 0,
    totalSteps: 0,
  }) as unknown as WorkspaceTask

const menuOf = async (task: WorkspaceTask) => {
  const { wrapper } = mountTimeline([], [task])
  await flushPromises()
  await wrapper.get('button[aria-haspopup="menu"]').trigger('click')
  return wrapper
}

it('offers Start focus for a manual task that is waiting on you and starts a block for it', async () => {
  const wrapper = await menuOf(waitingTask('manual'))
  const item = wrapper.findAll('[role="menuitem"]').find((i) => i.text() === 'Start focus')!
  await item.trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('focus_start_task', { taskId: 'task-9' })
  wrapper.unmount()
})

it('shows why a focus block could not start for the task', async () => {
  invoke.mockImplementation(async (command) => {
    if (command === 'focus_start_task') throw 'You have no free time before your next commitment.'
    return {}
  })
  const wrapper = await menuOf(waitingTask('manual'))
  await wrapper
    .findAll('[role="menuitem"]')
    .find((i) => i.text() === 'Start focus')!
    .trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('no free time')
  wrapper.unmount()
})

it('does not offer Start focus for Rune tasks or finished work', async () => {
  for (const task of [waitingTask('agent'), waitingTask('manual', 'completed')]) {
    const wrapper = await menuOf(task)
    expect(wrapper.findAll('[role="menuitem"]').map((i) => i.text())).not.toContain('Start focus')
    wrapper.unmount()
  }
})