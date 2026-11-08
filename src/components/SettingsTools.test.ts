import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import SettingsTools from './SettingsTools.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))

it('loads Kit fields and sends secret updates to Kit without retaining the secret', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_configuration')
      return [
        {
          plugin: 'weather',
          key: 'api_key',
          kind: 'string',
          description: 'API key',
          required: true,
          secret: true,
          locked: false,
          value: '',
        },
      ]
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Load settings')!
    .trigger('click')
  await flushPromises()
  const secret = wrapper.get('input[aria-label="weather: api_key"]')
  await secret.setValue('test-secret')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Save')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith(
    'kit_update_configuration',
    expect.objectContaining({
      update: { plugin: 'weather', key: 'api_key', value: 'test-secret', shared: false },
    }),
  )
  expect((secret.element as HTMLInputElement).value).toBe('')
})

it('sends third-party server configuration to Kit', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_remotes') return []
    if (command === 'kit_token_present') return false
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper.get('input[aria-label="Server name"]').setValue('example')
  await wrapper.get('input[aria-label="Server endpoint"]').setValue('https://example.test/mcp')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Save server')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith(
    'kit_remote_update',
    expect.objectContaining({
      name: 'example',
      remote: {
        name: 'example',
        endpoint: 'https://example.test/mcp',
        protocol: '2026-07-28',
        token_env: null,
      },
    }),
  )
})

it('shows installed plugins and skills returned by Kit', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_token_present') return false
    if (command === 'kit_packages')
      return {
        plugins: [{ name: 'clock', version: '1.2' }],
        skills: [{ name: 'review', version: '2.0', author: 'Kit' }],
      }
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Load packages')!
    .trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('clock')
  expect(wrapper.text()).toContain('review')
  expect(wrapper.text()).toContain('1 MCPs and 1 skills installed.')
})

it('installs through Kit and reports the plugin restart requirement', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_token_present') return false
    if (command === 'kit_packages') return { plugins: [], skills: [] }
    if (command === 'kit_install_package')
      return { name: 'clock', version: '1.0', restart_required: true }
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper.get('input[aria-label="Package target"]').setValue('clock')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Install')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith(
    'kit_install_package',
    expect.objectContaining({
      package: { kind: 'mcp', target: 'clock', version: null, prefer_native: false },
    }),
  )
  expect(wrapper.text()).toContain('Restart Kit to load the MCP server.')
})

it('requires confirmation before removing a Kit skill', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_token_present') return false
    if (command === 'kit_packages')
      return { plugins: [], skills: [{ name: 'review', version: '1' }] }
    if (command === 'kit_package_operation') return { restart_required: false }
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Load packages')!
    .trigger('click')
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Remove')!
    .trigger('click')
  expect(api.invoke).not.toHaveBeenCalledWith('kit_package_operation', expect.anything())
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Confirm removal')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith(
    'kit_package_operation',
    expect.objectContaining({ action: 'remove', kind: 'skill', name: 'review' }),
  )
})