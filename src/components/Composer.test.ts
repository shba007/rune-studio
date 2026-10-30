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
  await wrapper.get('[aria-label="Add request"]').trigger('click')
  await flushPromises()
  expect(board.submit).toHaveBeenCalledWith('Summarize this supplied text independently')
  expect(startTask).not.toHaveBeenCalled()
  expect(wrapper.get('textarea').element.value).toBe('')
  expect(wrapper.find('[aria-label="Stop"]').exists()).toBe(true)
  await wrapper.get('textarea').setValue('A second desktop task')
  await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(board.submit).toHaveBeenCalledWith('A second desktop task')
  const analysisSubmit = vi.fn<(text: string) => Promise<boolean>>(async () => true)
  const cancelAnalysis = vi.fn<() => void>()
  await wrapper.setProps({ analysisSubmit, cancelAnalysis })
  await wrapper.get('textarea').setValue('Identify the human')
  await wrapper.get('[aria-label="Analyze"]').trigger('click')
  await flushPromises()
  expect(analysisSubmit).toHaveBeenCalledWith('Identify the human')
  expect(board.submit).toHaveBeenCalledTimes(2)
  expect(wrapper.get('textarea').element.value).toBe('')
  analysisSubmit.mockResolvedValueOnce(false)
  await wrapper.get('textarea').setValue('Keep this until a target is chosen')
  await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(wrapper.get('textarea').element.value).toBe('Keep this until a target is chosen')
  await wrapper.setProps({ analysisBusy: true })
  expect(wrapper.get('[aria-label="Cancel analysis"]').classes()).toContain('bg-attention')
  expect(wrapper.get('textarea').attributes('disabled')).toBeDefined()
  await wrapper.get('[aria-label="Cancel analysis"]').trigger('click')
  expect(cancelAnalysis).toHaveBeenCalledOnce()
  agent.health.value.tone = 'down'
  await flushPromises()
  expect(wrapper.get('[aria-label="Cancel analysis"]').attributes('disabled')).toBeUndefined()
  await wrapper.setProps({ analysisBusy: false })
  expect(wrapper.get('textarea').attributes('disabled')).toBeDefined()
  expect(wrapper.get('textarea').classes()).toContain('placeholder:text-rune-text-3')
  expect(wrapper.get('[aria-label="Analyze"]').attributes('disabled')).toBeDefined()
  const submitted = analysisSubmit.mock.calls.length
  await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
  expect(analysisSubmit).toHaveBeenCalledTimes(submitted)
  agent.health.value.tone = 'ok'
  await flushPromises()
  expect(wrapper.get('textarea').attributes('disabled')).toBeUndefined()
  wrapper.unmount()
})
it('shows the approval mode and and toggles it', async () => {
  const saveSettings = vi.fn<(next: unknown) => Promise<boolean>>(async () => true)
  const agent = {
    state: ref('idle'),
    isRunning: ref(false),
    mode: ref('open'),
    focusTick: ref(0),
    settings: ref({ externalAgent: false, askEveryAction: false }),
    health: ref({ tone: 'ok' }),
    notice: ref(''),
    noticeKind: ref('info'),
    statusText: ref(''),
    saveSettings,
    stop: vi.fn<() => void>(),
    toggleListening: vi.fn<() => void>(),
  } as unknown as Agent
  const wrapper = mount(Composer, { props: { agent } })
  const mode = wrapper.get('[data-testid="approval-mode"]')
  expect(mode.text()).toBe('Auto')
  await mode.trigger('click')
  expect(saveSettings).toHaveBeenCalledWith(expect.objectContaining({ askEveryAction: true }))
})