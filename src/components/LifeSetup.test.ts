import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import LifeSetup from './LifeSetup.vue'

const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

beforeEach(() => {
  localStorage.clear()
  invoke
    .mockReset()
    .mockImplementation(async (command) =>
      command === 'workspace_goal_modes' ? ['manual', 'auto'] : 'new-goal',
    )
})

it('defaults to Auto on the left and starts only for the displayed, explicitly confirmed domain', async () => {
  const wrapper = mount(LifeSetup)
  await flushPromises()
  const choices = wrapper.findAll('[role="radio"]')
  expect(choices.map((button) => button.text())).toEqual(['Auto', 'Manual'])
  expect(choices[0]!.attributes('aria-checked')).toBe('true')
  expect(wrapper.find('textarea').exists()).toBe(false)
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Skip')!
    .trigger('click')
  expect(wrapper.get('h2').text()).toBe('Mental & Emotional Regulation')
  expect(wrapper.find('textarea').exists()).toBe(false)
  expect(wrapper.get('button[type="submit"]').text()).toBe(
    'Start Mental & Emotional Regulation discovery',
  )
  expect(invoke.mock.calls.some(([command]) => command === 'workspace_new_goal')).toBe(false)
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('workspace_new_goal', {
    text: null,
    domain: 'emotional',
    mode: 'auto',
    subDomains: [],
  })
  await choices[1]!.trigger('click')
  expect(wrapper.find('textarea').exists()).toBe(true)
  wrapper.unmount()
})

it('keeps Auto unavailable when capability discovery fails', async () => {
  invoke.mockRejectedValue(new Error('Unknown command'))
  const wrapper = mount(LifeSetup)
  await flushPromises()
  expect(
    wrapper
      .findAll('[role="radio"]')
      .find((button) => button.text() === 'Auto')!
      .attributes('disabled'),
  ).toBeDefined()
  expect(wrapper.find('textarea').exists()).toBe(true)
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  wrapper.unmount()
})