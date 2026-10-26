import { flushPromises, mount } from '@vue/test-utils'
import { computed, ref } from 'vue'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import type { WorkspaceTask } from '@/types/workspace'
import TaskReminder from './TaskReminder.vue'
const harness = vi.hoisted(() => ({
  action: vi.fn<() => Promise<void>>(),
  open: vi.fn<() => void>(),
}))
const due = ref<WorkspaceTask[]>([])
vi.mock('@/composables/useWorkspaceTasks', () => ({
  useWorkspaceTasks: () => ({
    due: computed(() => due.value),
    pending: ref(false),
    error: ref(''),
    action: harness.action,
  }),
}))
function reminder(executionMode: WorkspaceTask['executionMode']): WorkspaceTask {
  return {
    id: 'a',
    revision: 1,
    title: 'Review my screen',
    class: 'general',
    parentGoalId: null,
    createdMs: 0,
    schedule: null,
    paused: false,
    error: null,
    completedSteps: 0,
    totalSteps: 0,
    executionMode,
    occurrences: [
      { id: 'o', dueMs: 1, state: 'pending', runId: null, reported: false, completedMs: null },
    ],
  }
}
const agent = () =>
  ({ mode: ref('open'), setMode: harness.open, openWorkspace: harness.open }) as unknown as Agent
beforeEach(() => {
  vi.clearAllMocks()
  due.value = []
})
it('never dispatches a due task until the owner clicks Run now', async () => {
  due.value = [reminder('agent')]
  const wrapper = mount(TaskReminder, { props: { agent: agent() } })
  await flushPromises()
  expect(harness.action).not.toHaveBeenCalled()
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Run now')!
    .trigger('click')
  expect(harness.action).toHaveBeenCalledWith(due.value[0], 'run')
  wrapper.unmount()
})
it('asks for a new analysis capture and separately accepts reported completion', async () => {
  due.value = [reminder('analysis')]
  const wrapper = mount(TaskReminder, { props: { agent: agent() } })
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Choose capture')!
    .trigger('click')
  expect(harness.action).toHaveBeenCalledWith(due.value[0], 'analysis')
  expect(harness.action).not.toHaveBeenCalledWith(due.value[0], 'run')
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Already done')!
    .trigger('click')
  expect(harness.action).toHaveBeenLastCalledWith(due.value[0], 'done')
  wrapper.unmount()
})