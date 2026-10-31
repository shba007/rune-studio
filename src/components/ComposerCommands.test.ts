import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import type { TaskBoardModel } from '@/composables/useTaskBoard'
import Composer from './Composer.vue'
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<string>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

function show(externalAgent = false, running = false) {
  const analyzeScreen = vi.fn<() => Promise<void>>(async () => {})
  const openWorkspace = vi.fn<(section: string) => Promise<void>>(async () => {})
  const openSettings = vi.fn<() => Promise<void>>(async () => {})
  const submit = vi.fn<(text: string) => Promise<boolean>>(async () => true)
  const agent = {
    state: ref(running ? 'acting' : 'idle'),
    isRunning: ref(running),
    mode: ref('open'),
    focusTick: ref(0),
    settings: ref({ externalAgent }),
    health: ref({ tone: 'ok' }),
    notice: ref(''),
    noticeKind: ref('info'),
    statusText: ref('Ready'),
    screenRequest: ref({ status: 'idle', waiting: false }),
    analyzeScreen,
    openWorkspace,
    openSettings,
    toggleListening: vi.fn<() => void>(),
    stop: vi.fn<() => void>(),
  } as unknown as Agent
  const board = {
    requests: ref([]),
    notice: ref(''),
    pending: ref(false),
    submit,
  } as unknown as TaskBoardModel
  const wrapper = mount(Composer, { props: { agent, board } })
  return { wrapper, agent, analyzeScreen, openWorkspace, openSettings, submit }
}

it('suggests local commands and opens Goals using keyboard selection without planning a task', async () => {
  const { wrapper, openWorkspace, submit } = show()
  const field = wrapper.get('textarea')
  await field.setValue('/')
  expect(wrapper.findAll('[role="option"]')).toHaveLength(6)
  await field.trigger('keydown', { key: 'ArrowDown' })
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(openWorkspace).toHaveBeenCalledWith('goals')
  expect(submit).not.toHaveBeenCalled()
  expect(field.element.value).toBe('')
  expect(wrapper.find('[role="listbox"]').exists()).toBe(false)
  wrapper.unmount()
})

it('opens the existing Visual Analysis flow through a filtered suggestion or exact command', async () => {
  const { wrapper, analyzeScreen, submit } = show()
  const field = wrapper.get('textarea')
  await field.setValue('/vis')
  expect(wrapper.findAll('[role="option"]')).toHaveLength(1)
  await wrapper.get('[role="option"]').trigger('click')
  await flushPromises()
  expect(analyzeScreen).toHaveBeenCalledOnce()
  await field.setValue('/visual-analysis')
  await wrapper.get('[aria-label="Run command"]').trigger('click')
  await flushPromises()
  expect(analyzeScreen).toHaveBeenCalledTimes(2)
  expect(submit).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('preserves API desktop exclusion and external-agent Visual Analysis routing', async () => {
  const { wrapper, agent, analyzeScreen, submit } = show(false, true)
  const field = wrapper.get('textarea')
  await field.setValue('/visual analysis')
  expect(wrapper.get('[role="option"]').attributes('disabled')).toBeDefined()
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(analyzeScreen).not.toHaveBeenCalled()
  expect(wrapper.get('[role="alert"]').text()).toContain('Finish or stop')
  agent.settings.value!.externalAgent = true
  await flushPromises()
  expect(field.attributes('disabled')).toBeUndefined()
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(analyzeScreen).toHaveBeenCalledOnce()
  await field.setValue('Do not send this to the API planner')
  await field.trigger('keydown', { key: 'Enter' })
  expect(submit).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('supports completion and Escape, keeps unknown commands local, and respects offline input', async () => {
  const { wrapper, agent, analyzeScreen, openSettings, openWorkspace, submit } = show()
  const field = wrapper.get('textarea')
  await field.setValue('/set')
  await field.trigger('keydown', { key: 'Tab' })
  expect(field.element.value).toBe('/settings')
  expect(openSettings).not.toHaveBeenCalled()
  await field.trigger('keydown', { key: 'Escape' })
  expect(wrapper.find('[role="listbox"]').exists()).toBe(false)
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(openSettings).toHaveBeenCalledOnce()
  await field.setValue('/unknown')
  await field.trigger('keydown', { key: 'Enter' })
  expect(wrapper.get('[role="alert"]').text()).toContain('Unknown command')
  expect(submit).not.toHaveBeenCalled()
  agent.health.value.tone = 'down'
  await field.setValue('/visual analysis')
  expect(field.attributes('disabled')).toBeUndefined()
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(analyzeScreen).not.toHaveBeenCalled()
  expect(wrapper.get('[role="alert"]').text()).toContain('needs the model server')
  await field.setValue('/goals')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(openWorkspace).toHaveBeenCalledWith('goals')
  await field.setValue('hello')
  await field.trigger('keydown', { key: 'Enter' })
  expect(submit).not.toHaveBeenCalled()
  wrapper.unmount()
})
const model = vi.hoisted(() => ({ answer: {} as unknown }))
const creates = () => invoke.mock.calls.filter(([c]) => c === 'workspace_task_create')
function mockModel() {
  invoke.mockReset().mockImplementation((command) => {
    if (command === 'workspace_task_zone') return Promise.resolve('UTC')
    if (command === 'schedule_interpret') return Promise.resolve(model.answer as string)
    return Promise.resolve('id')
  })
  model.answer = { kind: 'unclear', title: '', delaySeconds: 0, local: '' }
}

it('creates a manual reminder from /schedule and from an inline remind phrase', async () => {
  mockModel()
  const { wrapper, submit } = show()
  const field = wrapper.get('textarea')
  await field.setValue('/schedule')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(field.element.value).toBe('/schedule ')
  await field.setValue('/schedule in 1 min drink water')
  expect(wrapper.find('[role="listbox"]').exists()).toBe(false)
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(creates()[0]![1]).toMatchObject({
    title: 'Drink water',
    class: 'general',
    executionMode: 'manual',
    schedule: { repeat: 'once', zone: 'UTC' },
  })
  expect(field.element.value).toBe('')
  expect(wrapper.get('[role="alert"]').text()).toContain('Reminder set for')
  await field.setValue('remind in 1 min that i have to drink water')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(creates()).toHaveLength(2)
  expect(invoke.mock.calls.some(([c]) => c === 'schedule_interpret')).toBe(false)
  expect(submit).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('asks the model when the local parser cannot read the time', async () => {
  mockModel()
  model.answer = { kind: 'after', title: 'Drink water again', delaySeconds: 50, local: '' }
  const { wrapper, submit } = show()
  const field = wrapper.get('textarea')
  await field.setValue('/schedule after 30 + 20 s remind me to drink water again')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('schedule_interpret', {
    text: 'after 30 + 20 s remind me to drink water again',
  })
  expect(creates()[0]![1]).toMatchObject({ title: 'Drink water again' })
  expect(field.element.value).toBe('')
  model.answer = { kind: 'unclear', title: '', delaySeconds: 0, local: '' }
  await field.setValue('/schedule drink water')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('could not tell')
  expect(creates()).toHaveLength(1)
  await field.setValue('remind me to buy milk')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(creates()).toHaveLength(1)
  expect(submit).toHaveBeenCalledWith('remind me to buy milk')
  invoke.mockImplementation((command) =>
    command === 'schedule_interpret'
      ? Promise.reject(new Error('That time has already passed.'))
      : Promise.resolve('UTC'),
  )
  await field.setValue('remind me a week ago to nap')
  await field.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('already passed')
  expect(creates()).toHaveLength(1)
  wrapper.unmount()
})