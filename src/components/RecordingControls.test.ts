import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import type { ScreenRecording } from '@/types/protocol'
import RecordingControls from './RecordingControls.vue'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
  event: null as null | ((event: { payload: ScreenRecording }) => void),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<
    (name: string, callback: (event: { payload: ScreenRecording }) => void) => Promise<() => void>
  >(async (_, callback) => {
    harness.event = callback
    return () => {}
  }),
}))
afterEach(() => vi.clearAllMocks())
const idle: ScreenRecording = {
  id: 0,
  phase: 'idle',
  seconds: 0,
  monitor: null,
  path: null,
  error: null,
}

it('requires a selected target, supports save/discard, and presents the completed file', async () => {
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'screen_recording_state') return idle
    if (command === 'screen_insight_targets')
      return [{ target: { kind: 'window', id: 42 }, label: 'Window — Safe fixture' }]
    if (command === 'start_screen_recording') return { ...idle, id: 1, phase: 'recording' }
    return null
  })
  const wrapper = mount(RecordingControls)
  await flushPromises()
  const button = (label: string) => wrapper.findAll('button').find((b) => b.text() === label)!
  expect(button('Start recording').attributes('disabled')).toBeDefined()
  await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
  await button('Window — Safe fixture').trigger('click')
  await wrapper.get('input[type="number"]').setValue('3')
  await button('Start recording').trigger('click')
  await flushPromises()
  expect(harness.invoke).toHaveBeenCalledWith('start_screen_recording', {
    target: { kind: 'window', id: 42 },
    seconds: 3,
  })
  expect(wrapper.findAll('[role="status"]').some((p) => p.text().includes('Recording'))).toBe(true)
  await button('Stop and save').trigger('click')
  expect(harness.invoke).toHaveBeenCalledWith('stop_screen_recording', { discard: false })
  harness.event!({ payload: { ...idle, id: 1, phase: 'saving' } })
  await flushPromises()
  expect(button('Stop and save').attributes('disabled')).toBeDefined()
  await button('Cancel and discard').trigger('click')
  expect(harness.invoke).toHaveBeenCalledWith('stop_screen_recording', { discard: true })
  harness.event!({ payload: { ...idle, id: 1, phase: 'done', path: 'C:\\Recordings\\screen.mp4' } })
  await flushPromises()
  expect(wrapper.text()).toContain('Saved: C:\\Recordings\\screen.mp4')
  harness.event!({ payload: { ...idle, id: 2, phase: 'cancelled' } })
  await flushPromises()
  expect(wrapper.text()).toContain('No video saved')
  expect(wrapper.text()).not.toContain('screen.mp4')
  wrapper.unmount()
})

it('refresh closes the target list, shows progress, and clears a stale selection', async () => {
  let finish: ((value: unknown) => void) | undefined
  let calls = 0
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'screen_recording_state') return idle
    if (command === 'screen_insight_targets') {
      calls++
      if (calls === 1) return [{ target: { kind: 'window', id: 42 }, label: 'Window — Notepad' }]
      return new Promise((resolve) => {
        finish = resolve
      })
    }
    return null
  })
  const wrapper = mount(RecordingControls)
  await flushPromises()
  await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Window — Notepad')!
    .trigger('click')
  await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
  await wrapper.get('[aria-label="Refresh capture targets"]').trigger('click')
  expect(wrapper.find('[aria-label="Capture targets"]').exists()).toBe(false)
  expect(wrapper.get('[aria-label="Refresh capture targets"]').attributes('disabled')).toBeDefined()
  expect(wrapper.text()).toContain('Refreshing…')
  finish!([])
  await flushPromises()
  expect(calls).toBe(2)
  expect(wrapper.text()).toContain('0 capture targets available')
  expect(
    wrapper
      .findAll('button')
      .find((b) => b.text() === 'Start recording')!
      .attributes('disabled'),
  ).toBeDefined()
  wrapper.unmount()
})