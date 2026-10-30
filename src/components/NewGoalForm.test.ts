import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import NewGoalForm from './NewGoalForm.vue'

const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

const modes = vi.hoisted(() => ({ value: ['manual'] as string[] }))
beforeEach(() => {
  localStorage.clear()
  modes.value = ['manual']
  invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'workspace_goal_modes') {
      if (!modes.value.length) throw new Error('unknown command')
      return modes.value
    }
    return 'goal-id'
  })
})

it('sends the confirmed domain and sub-domains without writing local overrides', async () => {
  const wrapper = mount(NewGoalForm)
  await flushPromises()
  expect(wrapper.find('[data-testid="auto-note"]').exists()).toBe(true)
  await wrapper.get('textarea').setValue('Run 5 km without stopping')
  expect(wrapper.get('[data-testid="sub-domains"]').text()).toContain('Sleep & Circadian')
  await wrapper
    .findAll('[role="switch"]')
    .find((node) => node.attributes('aria-label') === 'Cardiorespiratory Capacity')!
    .trigger('click')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', {
    text: 'Run 5 km without stopping',
    domain: 'physical',
    mode: 'manual',
    subDomains: ['cardiorespiratory-capacity'],
  })
  expect(wrapper.emitted('saved')).toEqual([['goal', 'goal-id']])
  expect(JSON.parse(localStorage.getItem('rune.goalSubDomains') ?? '{}')).toEqual({})
  expect(JSON.parse(localStorage.getItem('rune.goalDomains') ?? '{}')).toEqual({})
  wrapper.unmount()
})

it('requires an explicit domain when Auto is offered, without switching modes on mount', async () => {
  modes.value = ['manual', 'auto']
  const wrapper = mount(NewGoalForm)
  await flushPromises()
  expect(wrapper.find('[data-testid="auto-note"]').exists()).toBe(false)
  expect(wrapper.find('textarea').exists()).toBe(true)
  await wrapper
    .findAll('[role="radio"]')
    .find((node) => node.text() === 'Auto')!
    .trigger('click')
  expect(wrapper.find('textarea').exists()).toBe(false)
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke.mock.calls.some(([command]) => command === 'workspace_new_goal')).toBe(false)
  await wrapper.get('button[aria-label="Domain"]').trigger('click')
  await wrapper
    .findAll('[role="option"]')
    .find((node) => node.text() === 'Physical Vitality')!
    .trigger('click')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', {
    text: null,
    domain: 'physical',
    mode: 'auto',
    subDomains: [],
  })
  wrapper.unmount()
})

it('does not file a Sanctuary suggestion while Sanctuary is hidden', async () => {
  const wrapper = mount(NewGoalForm)
  await flushPromises()
  await wrapper.get('textarea').setValue('Spend more time with my family')
  expect(wrapper.text()).toContain('Sanctuary domains are hidden')
  expect(wrapper.find('[data-testid="sub-domains"]').exists()).toBe(false)
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', {
    text: 'Spend more time with my family',
    domain: null,
    mode: 'manual',
    subDomains: [],
  })
  wrapper.unmount()
})

it('keeps Auto disabled and preserves legacy local choices when capability discovery fails', async () => {
  modes.value = []
  localStorage.setItem('rune.goalDomains', JSON.stringify({ old: 'career' }))
  const wrapper = mount(NewGoalForm)
  await flushPromises()
  const auto = wrapper.findAll('[role="radio"]').find((node) => node.text() === 'Auto')!
  expect(auto.attributes('disabled')).toBeDefined()
  await auto.trigger('click')
  expect(wrapper.find('textarea').exists()).toBe(true)
  await wrapper.get('textarea').setValue('Run 5 km')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', {
    text: 'Run 5 km',
    domain: null,
    mode: 'manual',
    subDomains: [],
  })
  expect(JSON.parse(localStorage.getItem('rune.goalDomains')!)).toEqual({ old: 'career' })
  wrapper.unmount()
})