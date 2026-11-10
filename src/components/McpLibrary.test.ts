import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it, vi } from 'vite-plus/test'
import McpLibrary from './McpLibrary.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))

const config = () => ({
  enabled: true,
  endpoint: 'http://127.0.0.1:47601/mcp',
  selected: ['time__get_current_time'],
})
let installed: Array<{ name: string; version: string }>
beforeEach(() => {
  installed = [{ name: 'rune-time', version: '0.3.0' }]
  api.invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'kit_packages') return { mcps: installed, skills: [] }
    if (command === 'kit_registry')
      return [
        { name: 'rune-fetch', description: 'Fetch pages', latest: '0.2.0', has_icon: true },
        { name: 'rune-time', description: 'Time', latest: '0.3.0', has_icon: true },
      ]
    if (command === 'kit_discover')
      return {
        tools: [
          { name: 'time__get_current_time' },
          ...(installed.some((i) => i.name === 'rune-fetch') ? [{ name: 'fetch__fetch' }] : []),
        ],
      }
    if (command === 'kit_install_package') {
      installed.push({ name: 'rune-fetch', version: '0.2.0' })
      return { version: '0.2.0' }
    }
    return null
  })
})
async function setup() {
  const wrapper = mount(McpLibrary, {
    props: {
      config: config(),
      'onUpdate:config': (value: unknown) => wrapper.setProps({ config: value as never }),
    },
  })
  await flushPromises()
  return wrapper
}
it('shows installed MCP servers with their enabled state and toggles their tools', async () => {
  const wrapper = await setup()
  const toggle = wrapper.get('[aria-label="Enable rune-time"]')
  expect(toggle.attributes('aria-checked')).toBe('true')
  await toggle.trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('kit_save', {
    config: { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] },
  })
  wrapper.unmount()
})
it('installs a registry MCP from Discover and enables its tools', async () => {
  const wrapper = await setup()
  await wrapper
    .findAll('[role="tab"]')
    .find((tab) => tab.text() === 'Discover')!
    .trigger('click')
  expect(wrapper.text()).toContain('Fetch pages')
  await wrapper.get('[aria-label="Install rune-fetch"]').trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('kit_install_package', {
    config: expect.anything(),
    package: { kind: 'mcp', target: 'rune-fetch', version: null, prefer_native: false },
  })
  expect(api.invoke).toHaveBeenCalledWith('kit_save', {
    config: expect.objectContaining({
      selected: expect.arrayContaining(['fetch__fetch', 'time__get_current_time']),
    }),
  })
  expect(wrapper.text()).toContain('rune-fetch 0.2.0 installed and enabled.')
  wrapper.unmount()
})
it('requires confirmation before removing an MCP server', async () => {
  const wrapper = await setup()
  await wrapper.get('[aria-label="Remove rune-time"]').trigger('click')
  expect(api.invoke.mock.calls.some(([name]) => name === 'kit_package_operation')).toBe(false)
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Confirm remove')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('kit_package_operation', {
    config: expect.anything(),
    action: 'remove',
    kind: 'mcp',
    name: 'rune-time',
  })
  wrapper.unmount()
})