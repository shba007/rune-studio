import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import OnboardingLocalModel from './OnboardingLocalModel.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))

it('offers only Holo models, preselects the estimate, and shows source and license', async () => {
  harness.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'embedded_model_candidates')
      return [
        { id: 'qwen3.5-4b-q4', name: 'Qwen', bytes: 1, license: 'Apache-2.0' },
        {
          id: 'holo3.1-4b-q4',
          name: 'Holo3.1 4B',
          bytes: 1,
          license: 'Apache-2.0',
          sourceUrl: 'https://huggingface.co/community/model',
          recommended: true,
          recommendationBasis: 'GPU estimate',
        },
        {
          id: 'holo4-27b-q4',
          name: 'Holo4 27B',
          bytes: 1,
          license: 'CC BY-NC 4.0 · non-commercial only',
          sourceUrl: 'https://huggingface.co/Hcompany/Holo4-27B-GGUF',
        },
      ]
    if (command === 'embedded_model_download_status') return { phase: '', downloaded: 0 }
  })
  const wrapper = mount(OnboardingLocalModel)
  await flushPromises()
  expect(wrapper.text()).not.toContain('Qwen')
  expect(wrapper.text()).toContain('non-commercial only')
  expect(wrapper.text()).toContain('https://huggingface.co/community/model')
  expect((wrapper.get('input[value="holo3.1-4b-q4"]').element as HTMLInputElement).checked).toBe(
    true,
  )
  expect(wrapper.text()).toContain('GPU estimate')
  await wrapper.get('input[value="holo4-27b-q4"]').setValue()
  expect(wrapper.text()).toContain('https://huggingface.co/Hcompany/Holo4-27B-GGUF')
  expect(wrapper.text()).not.toContain('https://huggingface.co/community/model')
  wrapper.unmount()
})
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))

it('downloads and starts the selected model without opening Settings', async () => {
  let phase = ''
  harness.invoke.mockReset().mockImplementation(async (command: string) => {
    if (command === 'embedded_model_candidates')
      return [
        {
          id: 'holo3.1-0.8b-q4',
          name: 'Small',
          bytes: 1000000000,
          license: 'MIT',
          revision: 'v1',
          manifest: 'reviewed',
          recommended: true,
        },
      ]
    if (command === 'embedded_model_download') {
      phase = 'complete'
      return
    }
    if (command === 'embedded_model_download_status')
      return { phase, model: 'holo3.1-0.8b-q4', downloaded: 0 }
  })
  const wrapper = mount(OnboardingLocalModel)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Download selected model')!
    .trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('embedded_model_download', {
    id: 'holo3.1-0.8b-q4',
    revision: 'v1',
    manifest: 'reviewed',
  })
  expect(harness.invoke).toHaveBeenCalledWith('embedded_model_use', { id: 'holo3.1-0.8b-q4' })
  expect(wrapper.emitted('ready')).toHaveLength(1)
  expect(harness.invoke).not.toHaveBeenCalledWith('open_settings_window', expect.anything())
  wrapper.unmount()
})