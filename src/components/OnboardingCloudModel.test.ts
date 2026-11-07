import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import OnboardingCloudModel from './OnboardingCloudModel.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<() => Promise<() => void>>(async () => () => {}),
}))

it('sets up a cloud connection and selects its model within onboarding', async () => {
  const existing = {
    id: 'local',
    name: 'Local',
    baseUrl: 'http://127.0.0.1:1',
    active: true,
    hasApiKey: false,
  }
  const cloud = {
    id: 'cloud',
    name: 'Cloud',
    baseUrl: 'https://example.com',
    active: false,
    hasApiKey: false,
  }
  const catalog = (revision: number, connections: object[]) => ({
    revision,
    activeId: revision < 3 ? 'local' : 'cloud',
    connections,
  })
  harness.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'connections_state') return catalog(1, [existing])
    if (command === 'connections_save') return catalog(2, [existing, cloud])
    if (command === 'connections_test')
      return {
        reachable: true,
        authOk: true,
        error: null,
        models: [{ id: 'vision', imageIn: 'yes', toolCalls: 'yes' }],
      }
    if (command === 'connections_activate') return catalog(3, [existing, cloud])
    if (command === 'get_settings') return { model: '', models: { agent: '' } }
  })
  const wrapper = mount(OnboardingCloudModel)
  await flushPromises()
  const inputs = wrapper.findAll('input')
  await inputs[0]!.setValue('Cloud')
  await inputs[1]!.setValue('https://example.com')
  await wrapper.get('form').trigger('submit')
  await flushPromises()
  await wrapper.get('select').setValue('vision')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Use this model')!
    .trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('connections_activate', { revision: 2, id: 'cloud' })
  expect(harness.invoke).toHaveBeenCalledWith('infer_select_agent', { model: 'vision' })
  expect(wrapper.emitted('ready')).toHaveLength(1)
  wrapper.unmount()
})