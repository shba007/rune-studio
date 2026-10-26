import { flushPromises, shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import WorkspaceWindow from './WorkspaceWindow.vue'
import TaskHistory from './TaskHistory.vue'
import FlightPlayer from './FlightPlayer.vue'
import DecisionInbox from './DecisionInbox.vue'
import KnowledgeOverview from './KnowledgeOverview.vue'

const harness = vi.hoisted(() => ({
  handler: null as null | ((event: { payload: { section?: string; runId?: string } }) => void),
  off: vi.fn<() => void>(),
  settings: vi.fn<() => void>(),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (_: string, handler: typeof harness.handler) => {
    harness.handler = handler
    return harness.off
  },
}))
vi.mock('@/composables/useAgent', () => ({
  useAgent: () => ({ notice: ref(null), openSettings: harness.settings }),
}))
vi.mock('@/composables/useDecisions', () => ({
  useDecisions: () => ({ entries: ref([]), refresh: async () => {} }),
}))
vi.mock('@/composables/useWorkspaceTasks', () => ({
  useWorkspaceTasks: () => ({ tasks: ref([]), refresh: async () => {} }),
}))
afterEach(() => {
  window.history.replaceState({}, '', '/')
  vi.clearAllMocks()
})

it('keeps task history mounted while switching workspace sections', async () => {
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  const history = wrapper.getComponent(TaskHistory)
  expect(history.props('active')).toBe(true)
  await wrapper
    .findAll('nav button')
    .find((button) => button.text() === 'Goals')!
    .trigger('click')
  expect(wrapper.get('h1').text()).toBe('Goals')
  expect(wrapper.getComponent(TaskHistory).vm).toBe(history.vm)
  expect(history.props('active')).toBe(false)
  harness.handler!({ payload: { section: 'tasks' } })
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Tasks')
  expect(history.props('active')).toBe(true)
  wrapper.unmount()
  expect(harness.off).toHaveBeenCalledOnce()
})

it('opens Goals without a Settings button in Knowledge', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=goals')
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Goals')
  expect(wrapper.getComponent(DecisionInbox).props('goalsOnly')).toBe(true)
  expect(wrapper.findAll('nav button').map((item) => item.text())).toEqual([
    'Overview',
    'Personalities',
    'Goals',
    'Tasks',
  ])
  expect(harness.settings).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('opens the Overview graph and follows its links without remounting task history', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=overview')
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  const history = wrapper.getComponent(TaskHistory)
  expect(history.props('active')).toBe(false)
  expect(wrapper.get('h1').text()).toBe('Overview')
  wrapper.getComponent(KnowledgeOverview).vm.$emit('navigate', 'personalities')
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Personalities')
  harness.handler!({ payload: { section: 'overview' } })
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Overview')
  expect(wrapper.getComponent(TaskHistory).vm).toBe(history.vm)
  wrapper.unmount()
})

it('opens Preview inside Knowledge and returns to the same task history', async () => {
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  const history = wrapper.getComponent(TaskHistory)
  const openTask = history.props('openTask') as (id: string) => void
  openTask('task-1')
  await flushPromises()
  const preview = wrapper.getComponent(FlightPlayer)
  expect(preview.props()).toMatchObject({ embedded: true, taskId: 'task-1' })
  expect(history.props('active')).toBe(false)
  preview.vm.$emit('back')
  await flushPromises()
  expect(wrapper.findComponent(FlightPlayer).exists()).toBe(false)
  expect(wrapper.getComponent(TaskHistory).vm).toBe(history.vm)
  expect(history.props('active')).toBe(true)
  harness.handler!({ payload: { section: 'tasks', runId: 'task-2' } })
  await flushPromises()
  expect(wrapper.getComponent(FlightPlayer).props('taskId')).toBe('task-2')
  wrapper.unmount()
})

it('preserves old Preview links as embedded task previews', async () => {
  window.history.replaceState({}, '', '/?view=flight&run=legacy-task')
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  expect(wrapper.getComponent(FlightPlayer).props()).toMatchObject({
    embedded: true,
    taskId: 'legacy-task',
  })
  wrapper.unmount()
})