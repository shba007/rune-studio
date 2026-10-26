import { shallowMount, flushPromises } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import WorkspaceTaskTimeline from './WorkspaceTaskTimeline.vue'
import TaskPreview from './TaskPreview.vue'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import type { FlightRun } from '@/config/flight'
vi.mock('@tauri-apps/api/core', () => ({ invoke: async () => ({}) }))
it('keeps the chronological timeline, includes previews, and opens the matching replay', async () => {
  const run: FlightRun = {
    id: 'run-a',
    task: 'Calculate 14 + 27',
    source: 'built-in',
    outcome: 'succeeded',
    startedUtc: '2026-10-27T07:00:00Z',
    endedUtc: null,
    steps: 1,
    verdict: null,
    thumb: 'frame.jpg',
  }
  const workspace = {
    tasks: ref([]),
    error: ref(''),
    pending: ref(false),
  } as unknown as WorkspaceTasksModel
  const wrapper = shallowMount(WorkspaceTaskTimeline, {
    props: { workspace, history: [run], goals: [], active: true },
  })
  await flushPromises()
  expect(wrapper.text()).not.toContain('Calendar')
  const preview = wrapper.getComponent(TaskPreview)
  expect(preview.props('run')).toEqual(run)
  preview.vm.$emit('open')
  expect(wrapper.emitted('open')).toEqual([['run-a']])
  expect(wrapper.get('[data-work-task="run-a"]').text()).toContain('Calculate 14 + 27')
  wrapper.unmount()
})