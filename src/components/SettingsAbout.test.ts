import { mount, flushPromises } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import SettingsAbout from './SettingsAbout.vue'
import { resetUpdaterForTests } from '@/composables/useUpdater'

vi.mock('@tauri-apps/api/app', () => ({ getVersion: async () => '0.8.2' }))
const invoke = vi.hoisted(() => vi.fn<(command: string, args?: unknown) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

it('loads installed Studio and CLI versions when About opens independently', async () => {
  resetUpdaterForTests()
  invoke.mockReset().mockResolvedValue({ cli: '0.8.1', platform: 'windows', arch: 'x86_64' })
  const agent = { isRunning: ref(false) } as unknown as Agent
  const wrapper = mount(SettingsAbout, { props: { agent } })
  await flushPromises()
  expect(wrapper.text()).toMatch(/Rune Studio\s*0\.8\.2/)
  expect(wrapper.text()).toMatch(/Studio CLI\s*0\.8\.1/)
  expect(wrapper.text()).toContain('x86_64')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Read the guide ↗')!
    .trigger('click')
  expect(invoke).toHaveBeenCalledWith('open_project_link', { target: 'docs' })
  wrapper.unmount()
})