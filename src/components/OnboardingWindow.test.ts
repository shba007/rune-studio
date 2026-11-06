import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import OnboardingWindow from './OnboardingWindow.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
  close: vi.fn<() => Promise<void>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ close: harness.close }) }))

beforeEach(() => {
  localStorage.clear()
  harness.invoke.mockReset()
  harness.close.mockReset().mockResolvedValue(undefined)
})

it('offers cloud setup when embedded Infer is unavailable in this build', async () => {
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'embedded_infer_available') return false
    if (command === 'onboarding_platform') return 'linux'
    return undefined
  })
  const wrapper = mount(OnboardingWindow)
  await flushPromises()
  expect(wrapper.text()).toContain('Computer-use support on this platform is still in development')
  await wrapper.get('footer button.setup-button').trigger('click')
  const local = wrapper
    .findAll('button')
    .find((button) => button.text().includes('On this computer'))!
  const cloud = wrapper
    .findAll('button')
    .find((button) => button.text().includes('Cloud or other server'))!
  expect(local.attributes('disabled')).toBeDefined()
  expect(cloud.attributes('aria-pressed')).toBe('true')
  await wrapper.get('footer button.setup-button').trigger('click')
  expect(wrapper.text()).toContain('Connect your model server')
  wrapper.unmount()
})

it('requires the selected server to be active and ready before finishing', async () => {
  let active = 'studio-embedded-infer'
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'get_settings')
      return {
        activeConnectionId: active,
        connectionsRevision: 3,
        models: { agent: 'model' },
        stopHotkey: 'Alt+S',
      }
    if (command === 'check_backend')
      return {
        connectionId: active,
        connectionRevision: 3,
        status: { reachable: true, authOk: true, error: null, models: [{ id: 'model' }] },
      }
    return undefined
  })
  const wrapper = mount(OnboardingWindow)
  await wrapper.get('footer button.setup-button').trigger('click')
  await wrapper
    .findAll('button')
    .find((button) => button.text().includes('Cloud or other server'))!
    .trigger('click')
  await wrapper.get('footer button.setup-button').trigger('click')
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('Connect your model server')
  expect(harness.invoke).not.toHaveBeenCalledWith('onboarding_complete')
  active = 'cloud-connection'
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Start with a small task')
  expect(wrapper.text()).toContain('Alt+S')
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('onboarding_complete')
  expect(harness.close).toHaveBeenCalledOnce()
  wrapper.unmount()
})

it('resumes an incomplete setup at its saved step', () => {
  localStorage.setItem('rune-onboarding-step', '2')
  localStorage.setItem('rune-onboarding-choice', 'local')
  const wrapper = mount(OnboardingWindow)
  expect(wrapper.text()).toContain('Prepare your local model')
  wrapper.unmount()
})

it('keeps setup state and allows retry if the window fails to close', async () => {
  localStorage.setItem('rune-onboarding-step', '2')
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'get_settings')
      return {
        activeConnectionId: 'studio-embedded-infer',
        connectionsRevision: 1,
        models: { agent: 'model' },
      }
    if (command === 'check_backend')
      return {
        connectionId: 'studio-embedded-infer',
        connectionRevision: 1,
        status: {
          reachable: true,
          authOk: true,
          error: null,
          models: [{ id: 'model', imageIn: 'yes', toolCalls: 'yes' }],
        },
      }
    return undefined
  })
  harness.close.mockRejectedValueOnce(new Error('close failed'))
  const wrapper = mount(OnboardingWindow)
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('close failed')
  expect(localStorage.getItem('rune-onboarding-step')).toBe('3')
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  expect(harness.close).toHaveBeenCalledTimes(2)
  expect(localStorage.getItem('rune-onboarding-step')).toBeNull()
  wrapper.unmount()
})

it('does not mark a model without vision as ready', async () => {
  localStorage.setItem('rune-onboarding-step', '3')
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'get_settings')
      return {
        activeConnectionId: 'studio-embedded-infer',
        connectionsRevision: 1,
        models: { agent: 'text' },
      }
    if (command === 'check_backend')
      return {
        connectionId: 'studio-embedded-infer',
        connectionRevision: 1,
        status: {
          reachable: true,
          authOk: true,
          error: null,
          models: [{ id: 'text', imageIn: 'no', toolCalls: 'yes' }],
        },
      }
    return undefined
  })
  const wrapper = mount(OnboardingWindow)
  expect(wrapper.text()).toContain('Prepare your local model')
  await wrapper.get('footer button.setup-button').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('supports image input and tool calls')
  expect(harness.invoke).not.toHaveBeenCalledWith('onboarding_complete')
  wrapper.unmount()
})