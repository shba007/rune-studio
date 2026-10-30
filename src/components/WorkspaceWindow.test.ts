import RuneNavItem from './RuneNavItem.vue'
import { flushPromises, shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import WorkspaceWindow from './WorkspaceWindow.vue'
import TaskHistory from './TaskHistory.vue'
import FlightPlayer from './FlightPlayer.vue'
import DecisionInbox from './DecisionInbox.vue'
import GoalsTable from './GoalsTable.vue'
import LifeGraph from './LifeGraph.vue'
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
  useDecisions: () => ({ entries: ref([]), error: ref(''), refresh: async () => {} }),
}))
vi.mock('@/composables/useWorkspaceTasks', () => ({
  useWorkspaceTasks: () => ({ tasks: ref([]), refresh: async () => {} }),
}))
afterEach(() => {
  window.history.replaceState({}, '', '/')
  vi.clearAllMocks()
})

it('keeps task history mounted while switching workspace sections', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=tasks')
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
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

it('opens on the Life graph with no page heading', async () => {
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
  await flushPromises()
  expect(wrapper.findComponent(LifeGraph).exists()).toBe(true)
  expect(wrapper.find('h1').exists()).toBe(false)
  wrapper.unmount()
})

it('keeps the goals empty message out of the Tasks section', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=tasks')
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Tasks')
  expect(wrapper.getComponent(DecisionInbox).props('quiet')).toBe(true)
  wrapper.unmount()
})

it('opens Goals without a Settings button in Workspace', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=goals')
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Goals')
  expect(wrapper.findComponent(GoalsTable).exists()).toBe(true)
  expect(wrapper.findAll('nav button').map((item) => item.text())).toEqual([
    'Life',
    'Mindsets',
    'Goals',
    'Tasks',
    'Help',
  ])
  expect(harness.settings).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('opens old Overview links as Goals and follows graph links without remounting task history', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=overview')
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
  await flushPromises()
  const history = wrapper.getComponent(TaskHistory)
  expect(history.props('active')).toBe(false)
  expect(wrapper.get('h1').text()).toBe('Goals')
  expect(wrapper.findComponent(KnowledgeOverview).exists()).toBe(false)
  await wrapper
    .findAll('nav button')
    .find((button) => button.text() === 'Help')!
    .trigger('click')
  expect(wrapper.get('h1').text()).toBe('Help')
  wrapper.getComponent(KnowledgeOverview).vm.$emit('navigate', 'mindsets')
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Mindsets')
  harness.handler!({ payload: { section: 'overview' } })
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Goals')
  expect(wrapper.getComponent(TaskHistory).vm).toBe(history.vm)
  wrapper.unmount()
})

it('opens Preview inside Workspace and returns to the same task history', async () => {
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
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
  const wrapper = shallowMount(WorkspaceWindow, { global: { stubs: { RuneNavItem } } })
  await flushPromises()
  expect(wrapper.getComponent(FlightPlayer).props()).toMatchObject({
    embedded: true,
    taskId: 'legacy-task',
  })
  wrapper.unmount()
})