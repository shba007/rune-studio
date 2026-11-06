import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import OnboardingLocalModel from './OnboardingLocalModel.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))

it('downloads and starts the selected model without opening Settings', async () => {
  let phase = ''
  harness.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'embedded_model_candidates')
      return [
        {
          id: 'small',
          name: 'Small',
          bytes: 1000000000,
          license: 'MIT',
          revision: 'v1',
          manifest: 'reviewed',
        },
      ]
    if (command === 'embedded_model_download') {
      phase = 'complete'
      return
    }
    if (command === 'embedded_model_download_status')
      return { phase, model: 'small', downloaded: 0 }
  })
  const wrapper = mount(OnboardingLocalModel)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Download')!
    .trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('embedded_model_download', {
    id: 'small',
    revision: 'v1',
    manifest: 'reviewed',
  })
  expect(harness.invoke).toHaveBeenCalledWith('embedded_model_use', { id: 'small' })
  expect(wrapper.emitted('ready')).toHaveLength(1)
  expect(harness.invoke).not.toHaveBeenCalledWith('open_settings_window', expect.anything())
  wrapper.unmount()
})