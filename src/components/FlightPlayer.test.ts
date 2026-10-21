import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'

import { makeCall, makeHeader, makeStep, stubResizeObserver } from './flightTestKit'
import FlightPlayer from './FlightPlayer.vue'

const invoke = vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, args?: Record<string, unknown>) => invoke(command, args),
  convertFileSrc: (path: string) => `asset://${path}`,
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn<() => Promise<() => void>>(async () => () => {}),
}))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ close: vi.fn<() => void>() }),
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
        return w.header === undefined ? makeHeader() : w.header
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
  stubResizeObserver(800, 450)
  document.body.innerHTML = ''
})

describe('FlightPlayer states', () => {
  it('says so when nothing has been recorded', async () => {
    world({ runs: [] })
    const wrapper = await open(null)
    expect(wrapper.find('[data-testid="empty"]').exists()).toBe(true)
  })

  it('shows one line when the record cannot be read, never a crash', async () => {
    world({ fail: 'The flight record is not available.' })
    const wrapper = await open(null)
    expect(wrapper.find('[data-testid="problem"]').text()).toContain('not available')
  })

  it('lists the sessions and opens one from the list', async () => {
    world({})
    const wrapper = await open(null)
    const rows = wrapper.findAll('[data-testid="run-row"]')
    expect(rows).toHaveLength(1)
    await rows[0]!.trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="task"]').text()).toBe('Open notepad')
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