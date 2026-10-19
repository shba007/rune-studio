import { beforeEach, describe, expect, it, vi } from 'vitest'

const check = vi.fn<(options?: { timeout?: number }) => Promise<unknown>>()
const relaunch = vi.fn<() => Promise<void>>()

vi.mock('@tauri-apps/plugin-updater', () => ({ check: (...args: unknown[]) => check(...args) }))
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: () => relaunch() }))
vi.mock('@tauri-apps/api/app', () => ({ getVersion: () => Promise.resolve('0.0.3') }))

import { resetUpdaterForTests, useUpdater } from './useUpdater'

type DownloadEvent =
  | { event: 'Started'; data: { contentLength?: number } }
  | { event: 'Progress'; data: { chunkLength: number } }
  | { event: 'Finished' }

function fakeUpdate(events: DownloadEvent[] = []) {
  return {
    version: '0.0.4',
    body: 'notes',
    downloadAndInstall: vi.fn<(on: (event: DownloadEvent) => void) => Promise<void>>(async (on) =>
      events.forEach(on),
    ),
  }
}

describe('useUpdater', () => {
  beforeEach(() => {
    resetUpdaterForTests()
    check.mockReset()
    relaunch.mockReset()
  })

  it('a quiet check that finds an update raises the prompt', async () => {
    check.mockResolvedValue(fakeUpdate())
    const updater = useUpdater()
    await updater.check(true)
    expect(updater.phase.value).toBe('available')
    expect(updater.latest.value?.version).toBe('0.0.4')
    expect(updater.prompt.value).toBe(true)
  })

  it('a manual check does not raise the prompt, and says when it is up to date', async () => {
    const updater = useUpdater()
    check.mockResolvedValueOnce(fakeUpdate())
    await updater.check()
    expect(updater.prompt.value).toBe(false)
    check.mockResolvedValueOnce(null)
    await updater.check()
    expect(updater.phase.value).toBe('uptodate')
  })

  it('a quiet check stays silent when offline, a manual one reports it', async () => {
    check.mockRejectedValue(new Error('offline'))
    const updater = useUpdater()
    await updater.check(true)
    expect(updater.phase.value).toBe('idle')
    expect(updater.error.value).toBeNull()
    await updater.check()
    expect(updater.phase.value).toBe('error')
  })

  it('installing reports progress, then restarts', async () => {
    const update = fakeUpdate([
      { event: 'Started', data: { contentLength: 200 } },
      { event: 'Progress', data: { chunkLength: 50 } },
      { event: 'Finished' },
    ])
    check.mockResolvedValue(update)
    const updater = useUpdater()
    await updater.check(true)
    const seen: (number | null)[] = []
    update.downloadAndInstall.mockImplementation(async (on: (event: DownloadEvent) => void) => {
      on({ event: 'Started', data: { contentLength: 200 } })
      on({ event: 'Progress', data: { chunkLength: 50 } })
      seen.push(updater.progress.value)
      on({ event: 'Finished' })
      expect(updater.phase.value).toBe('installing')
    })
    await updater.install()
    expect(seen).toEqual([0.25])
    expect(updater.progress.value).toBe(1)
    expect(relaunch).toHaveBeenCalledTimes(1)
    expect(updater.prompt.value).toBe(false)
  })

  it('does not install while a task is running or twice at once', async () => {
    const update = fakeUpdate()
    check.mockResolvedValue(update)
    const updater = useUpdater()
    await updater.check(true)
    await updater.install(true)
    expect(update.downloadAndInstall).not.toHaveBeenCalled()
    await Promise.all([updater.install(), updater.install()])
    expect(update.downloadAndInstall).toHaveBeenCalledTimes(1)
  })

  it('a failed install ends in an error, not a stuck ring', async () => {
    const update = fakeUpdate()
    update.downloadAndInstall.mockRejectedValue(new Error('disk full'))
    check.mockResolvedValue(update)
    const updater = useUpdater()
    await updater.check(true)
    await updater.install()
    expect(updater.phase.value).toBe('error')
    expect(updater.updating.value).toBe(false)
    expect(relaunch).not.toHaveBeenCalled()
  })
})
