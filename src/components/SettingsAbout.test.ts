import { mount, flushPromises } from '@vue/test-utils'
import { ref } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import SettingsAbout from './SettingsAbout.vue'
import { resetUpdaterForTests } from '@/composables/useUpdater'

vi.mock('@tauri-apps/api/app', () => ({ getVersion: async () => '0.8.2' }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: async () => ({ cli: '0.8.1' }) }))

it('loads installed Studio and CLI versions when About opens independently', async () => {
  resetUpdaterForTests()
  const agent = { isRunning: ref(false) } as unknown as Agent
  const wrapper = mount(SettingsAbout, { props: { agent } })
  await flushPromises()
  expect(wrapper.text()).toContain('Installed Studio 0.8.2')
  expect(wrapper.text()).toContain('CLI 0.8.1')
  wrapper.unmount()
})