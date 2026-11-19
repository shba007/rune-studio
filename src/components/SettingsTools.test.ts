import { flushPromises, mount } from '@vue/test-utils'
import { expect, it, vi } from 'vite-plus/test'
import SettingsTools from './SettingsTools.vue'

const api = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }))

it('shows Kit-owned sandbox workspaces and approval records', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_sandbox_inventory')
      return {
        workspace_only: true,
        plugins: [
          {
            plugin: 'example',
            workspace: '/sandbox/example',
            approvals: [{ path: '/files/report.txt', kind: 'file' }],
          },
          { plugin: 'new', workspace: null, approvals: [] },
        ],
      }
    if (command === 'kit_token_present') return false
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Load sandbox')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('kit_sandbox_inventory', {
    config: expect.objectContaining({ endpoint: 'http://127.0.0.1:47601/mcp' }),
  })
  expect(wrapper.text()).toContain('/sandbox/example')
  expect(wrapper.text()).toContain('/files/report.txt')
  expect(wrapper.text()).toContain('Workspace not created')
  expect(wrapper.text()).toContain('do not by themselves change runtime permissions')
  wrapper.unmount()
})

it('requires explicit confirmation before applying or restoring a reviewed item', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_changes_inventory')
      return {
        plugins: [
          {
            plugin: 'example',
            reviews: [
              {
                path: '/original',
                staged_path: '/staged',
                deleted: false,
                baseline: 'old-hash',
                staged_digest: 'new-hash',
                approval: 'a'.repeat(64),
              },
            ],
            recoveries: [
              {
                path: '/original',
                id: 'recovery-1',
                backup_path: '/backup',
                completed: null,
                approval: 'b'.repeat(64),
              },
            ],
          },
        ],
      }
    if (command === 'kit_token_present') return false
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Load changes')!
    .trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('example: Modify')
  expect(wrapper.text()).toContain('/staged')
  expect(wrapper.text()).toContain('old-hash')
  expect(wrapper.text()).toContain('Unresolved backup')
  expect(wrapper.text()).toContain('/backup')
  expect(wrapper.text()).toContain('Kit rejects stale reviews')
  expect(api.invoke).toHaveBeenCalledWith('kit_changes_inventory', expect.anything())
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Review apply')!
    .trigger('click')
  expect(api.invoke.mock.calls.some(([command]) => command === 'kit_change_action')).toBe(false)
  expect(wrapper.text()).toContain('Confirm file change')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Confirm apply')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith(
    'kit_change_action',
    expect.objectContaining({
      action: { plugin: 'example', action: 'apply', approval: 'a'.repeat(64) },
    }),
  )
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Review restore')!
    .trigger('click')
  expect(wrapper.text()).toContain('Confirm backup restore')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Confirm restore')!
    .trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith(
    'kit_change_action',
    expect.objectContaining({
      action: { plugin: 'example', action: 'restore', approval: 'b'.repeat(64) },
    }),
  )
  wrapper.unmount()
})

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
const kitEndpoint = 'http://127.0.0.1:47601/mcp'
function folderApi(
  initial: string | null,
  picked: string | null = null,
  failure?: string,
  external = false,
) {
  let current = initial
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command, args) => {
    if (command === 'kit_settings') return { enabled: true, endpoint: kitEndpoint, selected: [] }
    if (command === 'kit_packages')
      return { mcps: [{ name: 'rune-filesystem', version: '1' }], skills: [] }
    if (command === 'kit_registry') return []
    if (command === 'kit_token_present') return false
    if (command === 'kit_discover') return { version: 'Kit', tools: [] }
    if (command === 'kit_plugin_folder') {
      if (external) throw 'Kit is not embedded; set the folder on your Kit server'
      return current
    }
    if (command === 'pick_folder') return picked
    if (command === 'kit_set_plugin_folder') {
      if (failure) throw failure
      current = String(args?.value)
      return null
    }
  })
}
it.each([null, 'D:/Documents'])('shows the current filesystem folder: %s', async (current) => {
  folderApi(current)
  const wrapper = mount(SettingsTools)
  await flushPromises()
  const row = wrapper.get('[data-testid="filesystem-folder"]')
  expect(row.text()).toContain(current || 'No folder set')
  expect(row.get('button').attributes('disabled')).toBeUndefined()
  expect(row.find('input').exists()).toBe(false)
  expect(api.invoke).toHaveBeenCalledWith('kit_plugin_folder', {
    endpoint: kitEndpoint,
    plugin: 'rune-filesystem',
  })
  wrapper.unmount()
})
it('cancelling the native folder dialog saves nothing', async () => {
  folderApi(null)
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper.get('[data-testid="filesystem-folder"] button').trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('pick_folder')
  expect(api.invoke.mock.calls.some(([command]) => command === 'kit_set_plugin_folder')).toBe(false)
  expect(wrapper.get('[data-testid="filesystem-folder"]').text()).toContain('No folder set')
  wrapper.unmount()
})
it('sends only the dialog path unchanged, reloads the folder and discovers again after saving', async () => {
  const picked = 'D:/Chosen folder/../Chosen folder'
  folderApi(null, picked)
  const wrapper = mount(SettingsTools)
  await flushPromises()
  const discovers = api.invoke.mock.calls.filter(([command]) => command === 'kit_discover').length
  await wrapper.get('[data-testid="filesystem-folder"] button').trigger('click')
  await flushPromises()
  expect(api.invoke).toHaveBeenCalledWith('kit_set_plugin_folder', {
    endpoint: kitEndpoint,
    plugin: 'rune-filesystem',
    key: 'allowed_dir',
    value: picked,
  })
  expect(wrapper.get('[data-testid="filesystem-folder"]').text()).toContain(picked)
  expect(
    wrapper.findAll('[role="status"]').some((row) => row.text().includes('Folder saved by Kit.')),
  ).toBe(true)
  expect(
    api.invoke.mock.calls.filter(([command]) => command === 'kit_discover').length,
  ).toBeGreaterThan(discovers)
  wrapper.unmount()
})
it('shows the exact Kit save error in the existing status line and keeps the old folder', async () => {
  folderApi('D:/Previous', 'D:/Chosen', 'Configuration is busy; retry')
  const wrapper = mount(SettingsTools)
  await flushPromises()
  await wrapper.get('[data-testid="filesystem-folder"] button').trigger('click')
  await flushPromises()
  expect(
    wrapper.findAll('[role="status"]').some((row) => row.text() === 'Configuration is busy; retry'),
  ).toBe(true)
  expect(wrapper.get('[data-testid="filesystem-folder"]').text()).toContain('D:/Previous')
  wrapper.unmount()
})
it('disables the picker and shows the server-owned folder reason for external Kit', async () => {
  folderApi(null, null, undefined, true)
  const wrapper = mount(SettingsTools)
  await flushPromises()
  const row = wrapper.get('[data-testid="filesystem-folder"]')
  expect(row.text()).toContain('Kit is not embedded; set the folder on your Kit server')
  expect(row.get('button').attributes('disabled')).toBeDefined()
  expect(api.invoke.mock.calls.some(([command]) => command === 'pick_folder')).toBe(false)
  wrapper.unmount()
})
it('keeps agent file changes off until turned on, and saves the choice with the selected write tools', async () => {
  api.invoke.mockReset()
  api.invoke.mockImplementation(async (command: string) => {
    if (command === 'kit_settings')
      return { enabled: true, endpoint: 'http://127.0.0.1:47601/mcp', selected: [] }
    if (command === 'kit_discover')
      return {
        version: 'Kit',
        tools: [],
        blocked: [
          {
            name: 'rune-filesystem__write_file',
            description: 'Write a file',
            kind: 'writes',
            reason: 'changes files',
          },
          { name: 'x__other', description: 'o', kind: 'unavailable', reason: 'no' },
        ],
      }
    if (command === 'kit_token_present') return false
  })
  const wrapper = mount(SettingsTools)
  await flushPromises()
  expect(wrapper.text()).not.toContain('Let the agent change files')
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Refresh / discover')!
    .trigger('click')
  await flushPromises()
  expect(wrapper.text()).toContain('Let the agent change files')
  expect(wrapper.text()).toContain('rune-filesystem__write_file')
  expect(wrapper.text()).not.toContain('x__other')
  const switches = () => wrapper.findAll('[role="switch"]')
  const saves = () => api.invoke.mock.calls.filter(([command]) => command === 'kit_save')
  const before = saves().length
  const write = switches().find(
    (item) => item.attributes('aria-label') === 'rune-filesystem__write_file',
  )!
  expect(write.attributes('disabled')).toBeDefined()
  await switches()
    .find((item) => item.attributes('aria-label') === 'Let the agent change files')!
    .trigger('click')
  await flushPromises()
  await switches()
    .find((item) => item.attributes('aria-label') === 'rune-filesystem__write_file')!
    .trigger('click')
  await flushPromises()
  const last = saves().at(-1)![1] as { config: { agentWrites: boolean; selected: string[] } }
  expect(saves().length).toBeGreaterThan(before)
  expect(last.config.agentWrites).toBe(true)
  expect(last.config.selected).toContain('rune-filesystem__write_file')
  wrapper.unmount()
})