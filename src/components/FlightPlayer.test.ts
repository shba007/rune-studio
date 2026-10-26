import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'

import { makeCall, makeHeader, makeStep, stubResizeObserver } from './flightTestKit'
import FlightPlayer from './FlightPlayer.vue'

const invoke = vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>()
const nativeWindow = vi.hoisted(() => ({ close: vi.fn<() => void>() }))
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, args?: Record<string, unknown>) => invoke(command, args),
  convertFileSrc: (path: string) => `asset://${path}`,
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<() => Promise<() => void>>(async () => () => {}),
}))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => nativeWindow,
}))
vi.mock('@videojs/html/video/player', () => ({}))
vi.mock('@videojs/html/ui/container', () => ({}))

const RUN = {
  id: 'r1',
  task: 'Open notepad',
  source: 'built-in',
  outcome: 'succeeded',
  startedUtc: '2026-10-20T10:00:00Z',
  endedUtc: '2026-10-20T10:00:30Z',
  steps: 3,
  verdict: null,
  thumb: null,
}

interface World {
  history?: unknown
  mediaHeaders?: Record<string, unknown>
  runs?: unknown
  header?: unknown
  steps?: unknown[]
  calls?: unknown[]
  file?: (relative: string) => Promise<unknown>
  fail?: string
  developer?: boolean
}

function world(w: World) {
  invoke.mockImplementation(async (command, args) => {
    if (w.fail) throw w.fail
    switch (command) {
      case 'get_settings':
        return { developerMode: w.developer ?? true }
      case 'flight_runs':
        return w.runs ?? [RUN]
      case 'flight_run':
        if (w.mediaHeaders?.[String(args?.id)]) return w.mediaHeaders[String(args?.id)]
        return w.header === undefined ? makeHeader() : w.header
      case 'flight_task_history':
        return w.history ?? []
      case 'flight_steps':
        return w.steps ?? [makeStep(1, 0), makeStep(2, 10), makeStep(3, 20)]
      case 'flight_events':
        return []
      case 'flight_calls':
        return w.calls ?? []
      case 'flight_file':
        return w.file ? w.file(String(args?.relative)) : String(args?.relative)
      default:
        return null
    }
  })
}

async function open(run: string | null = 'r1') {
  window.history.replaceState({}, '', run ? `/?view=flight&run=${run}` : '/?view=flight')
  const wrapper = mount(FlightPlayer, { attachTo: document.body })
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  invoke.mockReset()
  nativeWindow.close.mockClear()
  stubResizeObserver(800, 450)
  document.body.innerHTML = ''
})

describe('FlightPlayer states', () => {
  it('seeks the correct desktop video using local video time on the shared task clock', async () => {
    const origin = Date.parse('2026-10-20T10:00:00Z')
    const task = (id: number, start: number, end: number) => ({
      id,
      kind: 'desktop',
      text: `Desktop ${id}`,
      phase: 'done',
      runId: `desktop-${id}`,
      startedAt: origin + start,
      finishedAt: origin + end,
      steps: [
        { text: 'Type', phase: 'done', startedAt: origin + start + 5000, finishedAt: origin + end },
      ],
    })
    world({
      header: makeHeader({ id: 'request', source: 'task-request' }),
      history: [
        { revision: 1, atMs: origin, snapshot: { tasks: [] } },
        {
          revision: 2,
          atMs: origin + 40000,
          snapshot: { request: {}, tasks: [task(1, 5000, 15000), task(2, 20000, 35000)] },
        },
      ],
      mediaHeaders: {
        'desktop-1': makeHeader({
          id: 'desktop-1',
          startedUtc: '2026-10-20T10:00:05Z',
          durationSecs: 10,
          video: { file: 'first.mp4' },
        }),
        'desktop-2': makeHeader({
          id: 'desktop-2',
          startedUtc: '2026-10-20T10:00:20Z',
          durationSecs: 15,
          video: { file: 'second.mp4' },
        }),
      },
    })
    const seekVideo = vi.fn<(ms: number) => Promise<void>>(async () => {})
    const stageStub = defineComponent({
      setup(_, { expose }) {
        expose({ seek: seekVideo, setRate: vi.fn<(rate: number) => Promise<void>>(async () => {}) })
        return () => null
      },
    })
    window.history.replaceState({}, '', '/?view=flight&run=request')
    const wrapper = mount(FlightPlayer, { global: { stubs: { FlightStage: stageStub } } })
    await flushPromises()
    await wrapper.get('[aria-label="Desktop 2, step 1: Type, done"]').trigger('click')
    await flushPromises()
    expect(seekVideo).toHaveBeenLastCalledWith(5000)
    expect(wrapper.get('[data-testid="time"]').text()).toBe('0:25 / 0:40')
    expect(invoke).toHaveBeenCalledWith('flight_file', { id: 'desktop-2', relative: 'second.mp4' })
    wrapper.unmount()
  })
  it('opens a whole request with three task rows and resolves captures against its desktop child', async () => {
    const origin = Date.parse('2026-10-20T10:00:00Z')
    const task = (id: number, kind: string, text: string) => ({
      id,
      kind,
      text,
      phase: 'done',
      runId: kind === 'desktop' ? 'desktop' : `background-${id}`,
      startedAt: origin + 5000,
      finishedAt: origin + 25000,
      steps: [
        { text: 'Complete', phase: 'done', startedAt: origin + 10000, finishedAt: origin + 25000 },
      ],
    })
    world({
      header: makeHeader({ id: 'request', source: 'task-request' }),
      history: [
        { revision: 1, atMs: origin, snapshot: { tasks: [] } },
        {
          revision: 2,
          atMs: origin + 30000,
          snapshot: {
            request: { historyId: 'request' },
            tasks: [
              task(1, 'background', 'Calculate'),
              task(2, 'background', 'Translate'),
              task(3, 'desktop', 'Notepad'),
            ],
          },
        },
      ],
      mediaHeaders: {
        desktop: makeHeader({
          id: 'desktop',
          startedUtc: '2026-10-20T10:00:05Z',
          endedUtc: '2026-10-20T10:00:25Z',
          durationSecs: 20,
        }),
      },
      steps: [makeStep(1, 0, { observationFile: 'screenshots/one.png' })],
    })
    const wrapper = await open('request')
    expect(wrapper.findAll('[data-task-row]')).toHaveLength(3)
    expect(wrapper.find('[data-testid="no-steps"]').exists()).toBe(false)
    await wrapper.get('[aria-label="Translate, step 1: Complete, done"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="time"]').text()).toBe('0:10 / 0:30')
    expect(invoke).toHaveBeenCalledWith('flight_file', {
      id: 'desktop',
      relative: 'screenshots/one.png',
    })
    wrapper.unmount()
  })
  it('goes back to Sessions when no run is asked for', async () => {
    world({})
    await open(null)
    expect(invoke).toHaveBeenCalledWith('open_workspace', { section: 'tasks' })
  })
  it('keeps Preview open and reports a workspace navigation failure', async () => {
    world({})
    const wrapper = await open('r1')
    invoke.mockRejectedValueOnce('Could not open Goals & Tasks')
    await wrapper.get('[data-testid="back"]').trigger('click')
    await flushPromises()
    expect(nativeWindow.close).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('Could not open Goals & Tasks')
    wrapper.unmount()
  })

  it('shows one line when the record cannot be read, never a crash', async () => {
    world({ fail: 'The flight record is not available.' })
    const wrapper = await open('r1')
    expect(wrapper.find('[data-testid="problem"]').text()).toContain('not available')
  })

  it('says there is no such run', async () => {
    world({ header: null })
    const wrapper = await open('gone')
    expect(wrapper.find('[data-testid="problem"]').text()).toContain('no such run')
  })

  it('shows the header facts', async () => {
    world({
      header: makeHeader({
        retries: 2,
        verdict: { result: 'verified', evidence: 'seen', mode: 'background', error: null },
      }),
    })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="outcome"]').text()).toBe('succeeded')
    expect(wrapper.find('[data-testid="verdict"]').text()).toBe('verified')
    const meta = wrapper.find('[data-testid="meta"]').text()
    expect(meta).toMatch(/Stepss*3/)
    expect(meta).toMatch(/Retriess*2/)
    expect(meta).toMatch(/Durations*30 s/)
  })

  it('shows LIVE for a run that has not ended', async () => {
    world({ header: makeHeader({ endedUtc: null, outcome: null, durationSecs: null }) })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="live"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="outcome"]').exists()).toBe(false)
  })

  it('explains a run that took no steps', async () => {
    world({ steps: [] })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="no-steps"]').exists()).toBe(true)
  })

  it('shows an Insight capture and answer even though it has no action steps', async () => {
    world({
      steps: [],
      header: makeHeader({
        source: 'screen-insight',
        steps: 0,
        analysis: {
          answer: 'The port is already in use.',
          findings: [],
          model: 'vision',
          capturedUtc: '2026-10-20T10:00:00Z',
          captureFile: 'screenshots/capture.png',
          error: null,
        },
      }),
    })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="no-steps"]').exists()).toBe(false)
    expect(wrapper.get('img[alt="Saved visual analysis capture"]').attributes('src')).toBe(
      'asset://screenshots/capture.png',
    )
    expect(wrapper.get('[data-testid="analysis-answer"]').text()).toContain(
      'The port is already in use.',
    )
    await wrapper.get('img[alt="Saved visual analysis capture"]').trigger('error')
    expect(wrapper.get('[data-testid="analysis-capture-notice"]').text()).toContain(
      'deleted or moved',
    )
  })

  it('shows the answer and explains why older Insight sessions have no capture', async () => {
    world({
      steps: [],
      header: makeHeader({ source: 'screen-insight', steps: 0, summary: 'An older answer.' }),
    })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="no-steps"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="analysis-answer"]').text()).toBe('An older answer.')
    expect(wrapper.get('[data-testid="analysis-capture-notice"]').text()).toContain(
      'Older in-memory images cannot be restored',
    )
  })

  it('falls back to the screenshots and says why when the video file is missing', async () => {
    world({
      header: makeHeader({ video: { file: 'session.mp4', kind: 'storyboard' } }),
      file: async (relative) => {
        if (relative.endsWith('.mp4')) throw 'there is no such file'
        return relative
      },
      steps: [makeStep(1, 0, { observationFile: 'screenshots/1.jpg' })],
    })
    const wrapper = await open()
    expect(wrapper.find('video').exists()).toBe(false)
    expect(wrapper.find('[data-testid="note"]').text()).toContain('video file is missing')
    expect(wrapper.find('[data-testid="still"]').exists()).toBe(true)
  })

  it('says a run from before the video existed has none, and survives a deleted screenshot', async () => {
    world({
      file: async () => {
        throw 'there is no such file'
      },
      steps: [makeStep(1, 0, { observationFile: 'screenshots/1.jpg' })],
    })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="note"]').text()).toContain('no video')
    expect(wrapper.find('[data-testid="no-picture"]').text()).toContain('deleted')
  })
})

describe('FlightPlayer controls', () => {
  it('moves between steps', async () => {
    world({})
    const wrapper = await open()
    expect(wrapper.find('[data-testid="badge"]').text()).toMatch(/1 \/ 3/)
    await wrapper.find('[data-testid="next"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="badge"]').text()).toMatch(/2 \/ 3/)
    await wrapper.find('[data-testid="prev"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="badge"]').text()).toMatch(/1 \/ 3/)
  })

  it('keys: Right and Left change step, D opens the drawer, Esc closes it, T cycles the tabs', async () => {
    world({ steps: [makeStep(1, 0, { promptText: 'hello' }), makeStep(2, 10)] })
    const wrapper = await open()
    const press = async (key: string) => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }))
      await flushPromises()
    }
    await press('ArrowRight')
    expect(wrapper.find('[data-testid="badge"]').text()).toMatch(/2 \/ 2/)
    await press('ArrowLeft')
    expect(wrapper.find('[data-testid="badge"]').text()).toMatch(/1 \/ 2/)
    await press('d')
    expect(wrapper.find('[data-testid="drawer"]').attributes('aria-hidden')).toBe('false')
    await press('t')
    expect(wrapper.find('[role="tab"][aria-selected="true"]').text()).toBe('Prompt')
    await press('Escape')
    expect(wrapper.find('[data-testid="drawer"]').attributes('aria-hidden')).toBe('true')
  })

  it('masks typed text by default and the Mask secrets toggle shows it', async () => {
    world({
      steps: [
        makeStep(1, 0, {
          tool: 'type_text',
          argsJson: '{"text":"hunter2"}',
          reason: 'Type hunter2 in the field',
        }),
      ],
    })
    const wrapper = await open()
    const toggle = wrapper.find('[data-testid="typed-toggle"]')
    expect(toggle.text()).toContain('Mask secrets')
    expect(toggle.attributes('aria-pressed')).toBe('true')
    expect(wrapper.text()).not.toContain('hunter2')
    await toggle.trigger('click')
    expect(toggle.attributes('aria-pressed')).toBe('false')
  })

  it('has no Actions button, and hides the API tab outside developer mode', async () => {
    world({
      developer: false,
      steps: [makeStep(1, 0)],
      calls: [makeCall(1, 1)],
    })
    const wrapper = await open()
    expect(wrapper.find('[data-testid="layer-toggle"]').exists()).toBe(false)
    await wrapper.find('[data-testid="details"]').trigger('click')
    expect(wrapper.find('[data-testid="tab-api"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="raw-toggle"]').exists()).toBe(false)
  })

  it('shows the API tab only for a step that has model calls, with retries listed', async () => {
    world({
      steps: [makeStep(1, 0), makeStep(2, 10)],
      calls: [makeCall(1, 2, { outcome: 'retried', status: 503 }), makeCall(2, 2, { attempt: 2 })],
    })
    const wrapper = await open()
    await wrapper.find('[data-testid="details"]').trigger('click')
    expect(wrapper.find('[data-testid="tab-api"]').exists()).toBe(false)
    await wrapper.find('[data-testid="next"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="tab-api"]').exists()).toBe(true)
  })

  it('plays the screenshots on a virtual clock when there is no video', async () => {
    const frames: FrameRequestCallback[] = []
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) =>
      frames.push(callback),
    )
    vi.stubGlobal('cancelAnimationFrame', () => {})
    vi.spyOn(performance, 'now').mockReturnValue(0)
    try {
      world({})
      const wrapper = await open()
      await wrapper.find('[aria-label="Speed"] button:last-child').trigger('click')
      await wrapper.find('[data-testid="play"]').trigger('click')
      frames.shift()?.(500)
      await flushPromises()
      expect(wrapper.find('[data-testid="time"]').text()).toMatch(/^0:02 /)
      await wrapper.find('[data-testid="play"]').trigger('click')
    } finally {
      vi.unstubAllGlobals()
      vi.restoreAllMocks()
    }
  })
})