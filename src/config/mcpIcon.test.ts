import { beforeEach, expect, it, vi } from 'vite-plus/test'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))

import { clearMcpIcons, loadMcpIcon } from './mcpIcon'

beforeEach(() => {
  clearMcpIcons()
  harness.invoke.mockReset()
  harness.invoke.mockImplementation(async (command) =>
    command === 'kit_settings'
      ? { endpoint: 'http://127.0.0.1:47601/mcp' }
      : 'data:image/svg+xml;base64,AAA',
  )
})

it('asks Kit once per MCP with the saved Kit settings', async () => {
  expect(await loadMcpIcon('rune-time')).toBe('data:image/svg+xml;base64,AAA')
  expect(await loadMcpIcon('rune-time')).toBe('data:image/svg+xml;base64,AAA')
  expect(harness.invoke).toHaveBeenCalledWith('kit_icon', {
    config: { endpoint: 'http://127.0.0.1:47601/mcp' },
    name: 'rune-time',
  })
  expect(harness.invoke.mock.calls.filter(([command]) => command === 'kit_icon')).toHaveLength(1)
})

it('has no icon when Kit has none or cannot be reached, and retries after a failure', async () => {
  harness.invoke.mockRejectedValueOnce(new Error('down'))
  expect(await loadMcpIcon('rune-fetch')).toBeNull()
  expect(await loadMcpIcon('rune-fetch')).toBe('data:image/svg+xml;base64,AAA')
  expect(await loadMcpIcon(' ')).toBeNull()
})