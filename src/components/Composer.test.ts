import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import type { TaskBoardModel } from '@/composables/useTaskBoard'
import Composer from './Composer.vue'

it('submits requests without user scheduling controls while another task runs', async () => {
  const startTask = vi.fn<(text: string) => Promise<boolean>>(async () => true)
  const agent = {
    state: ref('acting'),
    isRunning: ref(true),
    mode: ref('task'),
    focusTick: ref(0),
    settings: ref({ externalAgent: false }),
    health: ref({ tone: 'up' }),
    notice: ref(''),
    noticeKind: ref('info'),
    statusText: ref('Working'),
    startTask,
    stop: vi.fn<() => void>(),
    toggleListening: vi.fn<() => void>(),
  } as unknown as Agent
  const board: TaskBoardModel = {
    tasks: ref([]),
    requests: ref([]),
    notice: ref(''),
    pending: ref(false),
    submit: vi.fn<(text: string) => Promise<boolean>>(async () => true),
    cancelRequest: vi.fn<(id: number) => Promise<void>>(async () => {}),
    cancel: vi.fn<(id: number) => Promise<void>>(async () => {}),
    dismiss: vi.fn<(id: number) => Promise<void>>(async () => {}),
  }
  const wrapper = mount(Composer, { props: { agent, board } })
  expect(wrapper.find('[aria-label="Task execution mode"]').exists()).toBe(false)
  await wrapper.get('textarea').setValue('Summarize this supplied text independently')
  await wrapper.get('[aria-label="Add goal"]').trigger('click')
  await flushPromises()
  expect(board.submit).toHaveBeenCalledWith('Summarize this supplied text independently')
  expect(startTask).not.toHaveBeenCalled()
  expect(wrapper.get('textarea').element.value).toBe('')
  expect(wrapper.find('[aria-label="Stop"]').exists()).toBe(true)
  await wrapper.get('textarea').setValue('A second desktop task')
  await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(board.submit).toHaveBeenCalledWith('A second desktop task')
})