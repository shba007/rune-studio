import { beforeEach, expect, it, vi } from 'vite-plus/test'
import { useCaptureTargets } from './useCaptureTargets'

const invoke = vi.hoisted(() => vi.fn<(command: string) => Promise<unknown>>())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

beforeEach(() => invoke.mockReset())

it('loads targets and prevents overlapping refreshes', async () => {
  let resolve!: (value: unknown) => void
  invoke.mockReturnValue(new Promise((done) => (resolve = done)))
  const capture = useCaptureTargets('No targets')
  const pending = capture.load()
  expect(capture.refreshing.value).toBe(true)
  expect(await capture.load()).toBe('busy')
  const targets = [{ target: { kind: 'screen', id: 1 }, label: 'Screen', primary: true }]
  resolve(targets)
  expect(await pending).toBe('loaded')
  expect(invoke).toHaveBeenCalledTimes(1)
  expect(capture.targets.value).toEqual(targets)
  expect(capture.refreshing.value).toBe(false)
})

it('reports empty and failed responses and clears the notice on retry', async () => {
  invoke
    .mockResolvedValueOnce([])
    .mockRejectedValueOnce(new Error('unavailable'))
    .mockResolvedValueOnce([{ target: { kind: 'window', id: 2 }, label: 'Window' }])
  const capture = useCaptureTargets('No targets')
  expect(await capture.load()).toBe('loaded')
  expect(capture.notice.value).toBe('No targets')
  expect(await capture.load()).toBe('failed')
  expect(capture.targets.value).toEqual([])
  expect(capture.notice.value).toContain('unavailable')
  expect(await capture.load()).toBe('loaded')
  expect(capture.notice.value).toBe('')
  expect(capture.targets.value[0]?.target.id).toBe(2)
})