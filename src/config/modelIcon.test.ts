import { beforeEach, expect, it, vi } from 'vite-plus/test'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))

import { GENERIC_MODEL_ICON, clearModelIcons, loadModelIcon } from './modelIcon'

beforeEach(() => {
  clearModelIcons()
  harness.invoke.mockReset()
})

it('asks Infer once per model and uses the icon it serves', async () => {
  harness.invoke.mockResolvedValue('data:image/svg+xml;base64,AAA')
  expect(await loadModelIcon('qwen3.5-4b-q4')).toBe('data:image/svg+xml;base64,AAA')
  expect(await loadModelIcon('qwen3.5-4b-q4')).toBe('data:image/svg+xml;base64,AAA')
  expect(harness.invoke).toHaveBeenCalledTimes(1)
  expect(harness.invoke).toHaveBeenCalledWith('infer_model_icon', { model: 'qwen3.5-4b-q4' })
})

it('falls back to the generic icon when Infer has none or cannot be reached', async () => {
  harness.invoke.mockResolvedValueOnce(null)
  expect(await loadModelIcon('holo3.1-4b-q4')).toBe(GENERIC_MODEL_ICON)
  harness.invoke.mockRejectedValueOnce(new Error('down'))
  expect(await loadModelIcon('other')).toBe(GENERIC_MODEL_ICON)
  harness.invoke.mockResolvedValueOnce('data:image/svg+xml;base64,BBB')
  expect(await loadModelIcon('other')).toBe('data:image/svg+xml;base64,BBB')
  expect(await loadModelIcon('  ')).toBe(GENERIC_MODEL_ICON)
})