import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import GoalWorkPanel from './GoalWorkPanel.vue'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'

const invoke = vi.hoisted(() =>
  vi.fn<(command: string, args?: unknown) => Promise<void>>().mockResolvedValue(undefined),
)
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

const entry: IntakeEntry = {
  id: 'goal-cognitive',
  revision: 4,
  text: 'Learn Rust',
  class: 'general',
  kind: 'goal',
  status: 'goal_ready',
  domain: 'cognitive',
  question: null,
  goal: null,
  approvedGoal: { outcome: 'Learn Rust' },
  error: null,
  taskId: null,
  outcomeMetric: { baseline: 0, current: 2, target: 10, unit: 'chapters', note: '', updatedAt: '' },
}
const workspace = { tasks: ref([]) } as unknown as WorkspaceTasksModel

beforeEach(() => {
  localStorage.clear()
  invoke.mockClear()
})

it('hides numeric progress and measurement controls for Sanctuary by default', () => {
  const wrapper = mount(GoalWorkPanel, { props: { entry, workspace } })
  expect(wrapper.text()).not.toContain('Outcome progress')
  expect(wrapper.text()).not.toContain('Plan progress')
  expect(wrapper.text()).not.toContain('Record outcome progress')
  expect(invoke).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('allows Sanctuary measurement when enabled and closes the form when disabled', async () => {
  localStorage.setItem('rune.showSanctuary', 'true')
  const wrapper = mount(GoalWorkPanel, { props: { entry, workspace } })
  expect(wrapper.text()).toContain('Outcome progress')
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Record outcome progress')!
    .trigger('click')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('goal_measurement', {
    id: entry.id,
    revision: 4,
    baseline: 0,
    current: 2,
    target: 10,
    unit: 'chapters',
    note: '',
  })
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Record outcome progress')!
    .trigger('click')
  window.dispatchEvent(
    new StorageEvent('storage', {
      key: 'rune.showSanctuary',
      newValue: 'false',
      storageArea: localStorage,
    }),
  )
  await flushPromises()
  expect(wrapper.find('form').exists()).toBe(false)
  expect(wrapper.text()).not.toContain('Outcome progress')
  wrapper.unmount()
})

it('uses a stored Engine domain despite a conflicting legacy Sanctuary override', () => {
  localStorage.setItem('rune.goalDomains', JSON.stringify({ [entry.id]: 'cognitive' }))
  const wrapper = mount(GoalWorkPanel, {
    props: { entry: { ...entry, domain: 'career' }, workspace },
  })
  expect(wrapper.text()).toContain('Outcome progress')
  wrapper.unmount()
})