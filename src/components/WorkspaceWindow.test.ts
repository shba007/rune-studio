import { flushPromises, shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import WorkspaceWindow from './WorkspaceWindow.vue'
import TaskHistory from './TaskHistory.vue'

const harness = vi.hoisted(() => ({
  handler: null as null | ((event: { payload: { section: string } }) => void),
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
afterEach(() => {
  window.history.replaceState({}, '', '/')
  vi.clearAllMocks()
})

it('keeps task history mounted while switching workspace sections', async () => {
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  const history = wrapper.getComponent(TaskHistory)
  expect(history.props('active')).toBe(true)
  await wrapper.findAll('nav button')[0]!.trigger('click')
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

it('opens the Goals section from its link and preserves access to Settings', async () => {
  window.history.replaceState({}, '', '/?view=workspace&section=goals')
  const wrapper = shallowMount(WorkspaceWindow)
  await flushPromises()
  expect(wrapper.get('h1').text()).toBe('Goals')
  expect(wrapper.text()).toContain('Goal planning is not available yet')
  await wrapper.findAll('nav button')[2]!.trigger('click')
  expect(harness.settings).toHaveBeenCalledOnce()
  wrapper.unmount()
})