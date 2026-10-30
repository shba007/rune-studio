import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import WorkspaceTaskTimeline from './WorkspaceTaskTimeline.vue'
import TaskPreview from './TaskPreview.vue'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import type { FlightRun } from '@/config/flight'
import type { WorkspaceTask } from '@/types/workspace'
vi.mock('@tauri-apps/api/core', () => ({ invoke: async () => ({}) }))

const now = Date.now()
const run = (id: string, task: string, outcome: string | null, startedMs: number): FlightRun => ({
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

function mountTimeline(history: FlightRun[], tasks: WorkspaceTask[] = []) {
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