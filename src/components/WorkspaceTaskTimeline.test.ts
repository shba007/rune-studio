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

it('groups the timeline by day, includes previews, and opens the matching replay', async () => {
  const { wrapper } = mountTimeline([
    run('run-a', 'Calculate 14 + 27', 'succeeded', now),
    run('run-b', 'Older job', 'failed', now - 3 * 86_400_000),
  ])
  await flushPromises()
  expect(wrapper.text()).not.toContain('Calendar')
  const titles = wrapper.findAll('h3').map((heading) => heading.text())
  expect(titles[0]).toBe('Today')
  expect(titles).toHaveLength(2)
  const preview = wrapper.getComponent(TaskPreview)
  expect(preview.props('run')).toMatchObject({ id: 'run-a' })
  preview.vm.$emit('open')
  expect(wrapper.emitted('open')).toEqual([['run-a']])
  expect(wrapper.get('[data-work-task="run-a"]').text()).toContain('Calculate 14 + 27')
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