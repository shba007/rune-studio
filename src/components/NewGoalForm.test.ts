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

it('starts a typed goal, suggests a domain and remembers it locally', async () => {
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
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', { text: 'Run 5 km without stopping' })
  expect(wrapper.emitted('saved')).toEqual([['goal', 'goal-id']])
  expect(JSON.parse(localStorage.getItem('rune.goalSubDomains')!)).toEqual({
    'goal-id': ['cardiorespiratory-capacity'],
  })
})

it('keeps Auto off until the backend offers it, then defaults to it and asks for a domain, not a goal', async () => {
  modes.value = ['manual', 'auto']
  const wrapper = mount(NewGoalForm)
  await flushPromises()
  expect(wrapper.find('[data-testid="auto-note"]').exists()).toBe(false)
  expect(wrapper.find('textarea').exists()).toBe(false)
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', {
    text: null,
    domain: 'physical',
    mode: 'auto',
    subDomains: [],
  })
})

it('does not file a Sanctuary suggestion while Sanctuary is hidden', async () => {
  const wrapper = mount(NewGoalForm)
  await flushPromises()
  await wrapper.get('textarea').setValue('Spend more time with my family')
  expect(wrapper.text()).toContain('Sanctuary domains are hidden')
  expect(wrapper.find('[data-testid="sub-domains"]').exists()).toBe(false)
})