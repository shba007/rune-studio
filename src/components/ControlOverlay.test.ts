import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { CaptureFeedback, OverlayInfo } from '@/types/protocol'
import ControlOverlay from './ControlOverlay.vue'

const harness = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: unknown }) => void>(),
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
  tick: null as null | ((frame: { timestamp: number }) => void),
  pause: vi.fn<() => void>(),
  resume: vi.fn<() => void>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<
    (name: string, handler: (event: { payload: unknown }) => void) => Promise<() => void>
  >(async (name, handler) => {
    harness.handlers.set(name, handler)
    return () => harness.handlers.delete(name)
  }),
}))
vi.mock('@vueuse/core', async (original) => ({
  ...(await original<typeof import('@vueuse/core')>()),
  useRafFn: (tick: (frame: { timestamp: number }) => void) => {
    harness.tick = tick
    return { pause: harness.pause, resume: harness.resume }
  },
}))

afterEach(() => {
  vi.restoreAllMocks()
  vi.useRealTimers()
  harness.handlers.clear()
})

it('shows thinking and speech beside the orb with an expanded scaled task window', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
  vi.spyOn(window, 'devicePixelRatio', 'get').mockReturnValue(2)
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null)
  let rect = { x: 1000, y: 100, width: 752, height: 672 }
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'pill_rect') return rect
    if (command === 'analysis_state') return EMPTY_SNAPSHOT
    if (command === 'get_settings')
      return { stopHotkey: 'Escape', models: { agent: 'qwen3.8-27b-Q4_K_M' } }
    return null
  })
  const wrapper = mount(ControlOverlay)
  await flushPromises()
  harness.handlers.get('thinking')!({
    payload: { text: 'Checking the current step', done: false, model: 'qwen3.8-27b-Q4_K_M' },
  })
  await flushPromises()
  expect(wrapper.text()).toContain('Checking the current step')
  expect(wrapper.text()).toContain('qwen3.8-27b-Q4_K_M · requested')
  const bubble = wrapper.get('.w-\\[280px\\]')
  expect((bubble.element as HTMLElement).style.right).toBe(
    `${window.innerWidth - 596 + window.screenX}px`,
  )
  expect((bubble.element as HTMLElement).style.bottom).toBe(
    `${window.innerHeight - 322 + window.screenY + 12}px`,
  )
  harness.handlers.get('task_done')!({ payload: { summary: 'Finished the task' } })
  await flushPromises()
  expect(wrapper.text()).toContain('Finished the task')
  expect(wrapper.text()).not.toContain('Checking the current step')
  rect = { x: 1624, y: 644, width: 128, height: 128 }
  await vi.advanceTimersByTimeAsync(100)
  await flushPromises()
  expect(wrapper.text()).toContain('Finished the task')
  expect((bubble.element as HTMLElement).style.right).toBe(
    `${window.innerWidth - 876 + window.screenX}px`,
  )
  expect((bubble.element as HTMLElement).style.bottom).toBe(
    `${window.innerHeight - 322 + window.screenY + 12}px`,
  )
  const answer = 'The total is 180. The average is 36. '.repeat(5)
  harness.handlers.get('task_answer')!({ payload: answer })
  await flushPromises()
  expect(wrapper.text()).toContain(answer.trim())
  expect(wrapper.text()).toContain('qwen3.8-27b-Q4_K_M')
  expect(wrapper.text()).not.toContain('configured')
  harness.handlers.get('state_changed')!({ payload: { state: 'idle' } })
  await flushPromises()
  expect(wrapper.text()).toContain(answer.trim())
  harness.handlers.get('task_answer')!({ payload: null })
  await flushPromises()
  expect(wrapper.text()).not.toContain(answer.trim())
  wrapper.unmount()
})

it('keeps full-screen analysis ripples running throughout reading and stops them on completion', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] })
  vi.spyOn(performance, 'now').mockReturnValue(1000)
  const context = {
    setTransform: vi.fn<(...args: number[]) => void>(),
    clearRect: vi.fn<(...args: number[]) => void>(),
    beginPath: vi.fn<() => void>(),
    arc: vi.fn<(...args: number[]) => void>(),
    fill: vi.fn<() => void>(),
  }
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    context as unknown as CanvasRenderingContext2D,
  )
  const snapshot = {
    ...EMPTY_SNAPSHOT,
    id: 'insight-reading',
    status: 'asking',
    monitor: { id: 1, x: 0, y: 0, width: 1024, height: 768, scaleFactor: 1, primary: true },
  }
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'overlay_info')
      return {
        x: 0,
        y: 0,
        width: 1024,
        height: 768,
        scaleFactor: 1,
        primary: true,
        mode: 'analysis',
      }
    if (command === 'analysis_state') return snapshot
    if (command === 'get_settings') return { stopHotkey: 'Escape' }
    return null
  })
  const wrapper = mount(ControlOverlay)
  await flushPromises()
  harness.pause.mockClear()
  harness.handlers.get('state_changed')!({ payload: { state: 'idle' } })
  await vi.advanceTimersByTimeAsync(700)
  expect(harness.pause).not.toHaveBeenCalled()
  context.arc.mockClear()
  harness.tick!({ timestamp: 5000 })
  expect(context.arc.mock.calls.length).toBeGreaterThan(0)
  context.arc.mockClear()
  harness.tick!({ timestamp: 8000 })
  expect(context.arc.mock.calls.length).toBeGreaterThan(0)
  harness.handlers.get('analysis_updated')!({ payload: { ...snapshot, status: 'done' } })
  await flushPromises()
  context.arc.mockClear()
  harness.tick!({ timestamp: 9000 })
  expect(context.arc).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('recovers a capture on first overlay load, draws it once, and fades without claiming control', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] })
  vi.spyOn(performance, 'now').mockReturnValue(1000)
  const context = {
    setTransform: vi.fn<(...args: number[]) => void>(),
    clearRect: vi.fn<(...args: number[]) => void>(),
    beginPath: vi.fn<() => void>(),
    arc: vi.fn<(...args: number[]) => void>(),
    fill: vi.fn<() => void>(),
  }
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    context as unknown as CanvasRenderingContext2D,
  )
  let ready!: (info: OverlayInfo) => void
  const info = new Promise<OverlayInfo>((resolve) => {
    ready = resolve
  })
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'overlay_info') return info
    if (command === 'analysis_state') return EMPTY_SNAPSHOT
    if (command === 'get_settings') return { stopHotkey: 'Escape' }
    return null
  })
  const wrapper = mount(ControlOverlay)
  await flushPromises()
  const feedback: CaptureFeedback = {
    serial: 1,
    monitor: { id: 1, x: 0, y: 0, width: 1024, height: 768, scaleFactor: 1, primary: true },
    remainingMs: 900,
  }
  harness.handlers.get('capture_feedback')!({ payload: feedback })
  ready({
    x: 0,
    y: 0,
    width: 1024,
    height: 768,
    scaleFactor: 1,
    primary: true,
    mode: 'none',
    captureFeedback: feedback,
  })
  await flushPromises()
  context.arc.mockClear()
  harness.tick!({ timestamp: 1000 })
  const once = context.arc.mock.calls.length
  expect(once).toBeGreaterThan(0)
  expect(wrapper.text()).not.toContain('is in control')
  expect(wrapper.find('.opacity-100').exists()).toBe(true)
  harness.handlers.get('capture_feedback')!({ payload: feedback })
  await flushPromises()
  context.arc.mockClear()
  harness.tick!({ timestamp: 1000 })
  expect(context.arc.mock.calls.length).toBe(once)
  await vi.advanceTimersByTimeAsync(900)
  expect(wrapper.find('.opacity-0').exists()).toBe(true)
  wrapper.unmount()
})

it('keeps recording ripples running beyond one capture wave and ends them when saving starts', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] })
  vi.spyOn(performance, 'now').mockReturnValue(1000)
  const context = {
    setTransform: vi.fn<(...args: number[]) => void>(),
    clearRect: vi.fn<(...args: number[]) => void>(),
    beginPath: vi.fn<() => void>(),
    arc: vi.fn<(...args: number[]) => void>(),
    fill: vi.fn<() => void>(),
  }
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(
    context as unknown as CanvasRenderingContext2D,
  )
  const recording = {
    id: 2,
    phase: 'recording',
    seconds: 4,
    monitor: { id: 1, x: 0, y: 0, width: 1024, height: 768, scaleFactor: 1, primary: true },
    path: null,
    error: null,
  }
  harness.invoke.mockImplementation(async (command: string) => {
    if (command === 'overlay_info')
      return {
        x: 0,
        y: 0,
        width: 1024,
        height: 768,
        scaleFactor: 1,
        primary: true,
        mode: 'none',
        recording,
      }
    if (command === 'analysis_state') return EMPTY_SNAPSHOT
    if (command === 'get_settings') return { stopHotkey: 'Escape' }
    return null
  })
  const wrapper = mount(ControlOverlay)
  await flushPromises()
  context.arc.mockClear()
  harness.tick!({ timestamp: 5000 })
  expect(context.arc.mock.calls.length).toBeGreaterThan(0)
  expect(wrapper.text()).toContain('Screen recording')
  expect(wrapper.text()).not.toContain('is in control')
  harness.handlers.get('screen_recording')!({ payload: { ...recording, phase: 'saving' } })
  await flushPromises()
  context.arc.mockClear()
  harness.tick!({ timestamp: 6000 })
  expect(context.arc).not.toHaveBeenCalled()
  expect(wrapper.find('.opacity-0').exists()).toBe(true)
  wrapper.unmount()
})