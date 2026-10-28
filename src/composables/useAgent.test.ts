import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import { useAgent, type Agent } from './useAgent'
import Composer from '@/components/Composer.vue'
import type { BackendProbe } from '@/types/protocol'

const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string) => Promise<unknown>>(),
  handlers: new Map<string, Set<(event: { payload: unknown }) => void>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
    const handlers = harness.handlers.get(name) ?? new Set()
    harness.handlers.set(name, handlers)
    handlers.add(handler)
    return () => handlers.delete(handler)
  },
}))
afterEach(() => {
  vi.useRealTimers()
  harness.handlers.clear()
  harness.invoke.mockReset()
})

it('shares connectivity between Settings and composer, rejects stale results, and recovers on polling', async () => {
  vi.useFakeTimers()
  window.history.replaceState({}, '', '/')
  let revision = 0
  let online = false
  const probe = (rev: number, reachable: boolean): BackendProbe => ({
    revision: rev,
    status: {
      reachable,
      authOk: true,
      missingModels: [],
      modelCapabilities: [],
      latencyMs: 1,
      error: reachable ? null : 'offline',
    },
  })
  const broadcast = (value: BackendProbe) =>
    harness.handlers.get('backend_status')?.forEach((handler) => handler({ payload: value }))
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'get_settings') return { externalAgent: false, model: 'vision' }
    if (command === 'check_backend') {
      const value = probe(++revision, online)
      broadcast(value)
      return value
    }
    return null
  })
  const agents: Agent[] = []
  const component = defineComponent({
    components: { Composer },
    setup() {
      const agent = useAgent()
      agents.push(agent)
      return { agent }
    },
    template: '<Composer :agent="agent" />',
  })
  const main = mount(component)
  const settings = mount(component)
  await flushPromises()
  expect(main.get('textarea').attributes('placeholder')).toContain('Offline')
  online = true
  await agents[1]!.refreshBackend()
  await flushPromises()
  expect(agents.every((agent) => agent.health.value.tone === 'ok')).toBe(true)
  expect(main.get('textarea').attributes('placeholder')).not.toContain('Offline')
  let finish!: (value: BackendProbe) => void
  const oldRevision = ++revision
  harness.invoke.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve
      }),
  )
  const old = agents[0]!.refreshBackend()
  broadcast(probe(++revision, true))
  finish(probe(oldRevision, false))
  await old
  expect(agents.every((agent) => agent.health.value.tone === 'ok')).toBe(true)
  online = false
  await vi.advanceTimersByTimeAsync(15000)
  await flushPromises()
  expect(main.get('textarea').attributes('placeholder')).toContain('Offline')
  online = true
  await vi.advanceTimersByTimeAsync(15000)
  await flushPromises()
  expect(main.get('textarea').attributes('placeholder')).not.toContain('Offline')
  main.unmount()
  settings.unmount()
})
it('clears the old endpoint and ignores its late status after settings change', async () => {
  let currentUrl = 'http://127.0.0.1:8080'
  const status = (revision: number, baseUrl: string, reachable: boolean): BackendProbe => ({
    revision,
    baseUrl,
    status: {
      reachable,
      authOk: true,
      missingModels: [],
      modelCapabilities: [],
      latencyMs: 1,
      error: reachable ? null : 'offline',
    },
  })
  let finishNew!: (value: BackendProbe) => void
  harness.invoke.mockImplementation(async (command) => {
    if (command === 'get_settings')
      return { externalAgent: false, model: 'vision', baseUrl: currentUrl }
    if (command === 'check_backend') {
      if (currentUrl.endsWith('9090'))
        return await new Promise<BackendProbe>((resolve) => {
          finishNew = resolve
        })
      return status(1, currentUrl, true)
    }
    return null
  })
  let agent!: Agent
  const wrapper = mount(
    defineComponent({
      setup() {
        agent = useAgent()
        return () => null
      },
    }),
  )
  await flushPromises()
  expect(agent.health.value.tone).toBe('ok')
  currentUrl = 'http://127.0.0.1:9090'
  harness.handlers.get('settings_changed')?.forEach((handler) => handler({ payload: null }))
  await flushPromises()
  expect(agent.settings.value?.baseUrl).toBe(currentUrl)
  expect(agent.health.value.tone).toBe('checking')
  harness.handlers
    .get('backend_status')
    ?.forEach((handler) => handler({ payload: status(2, 'http://127.0.0.1:8080', true) }))
  expect(agent.health.value.tone).toBe('checking')
  finishNew(status(3, currentUrl, true))
  await flushPromises()
  expect(agent.health.value.tone).toBe('ok')
  wrapper.unmount()
})
it('serializes window resizing and skips superseded and duplicate measurements', async () => {
  let finish: (() => void) | undefined
  const heights: number[] = []
  harness.invoke.mockImplementation((command, ...args: unknown[]) => {
    if (command !== 'fit_height') return Promise.resolve(null)
    heights.push((args[0] as { height: number }).height)
    return new Promise<void>((resolve) => {
      finish = resolve
    })
  })
  let agent!: Agent
  const wrapper = mount(
    defineComponent({
      setup() {
        agent = useAgent()
        return () => null
      },
    }),
  )
  await flushPromises()
  agent.mode.value = 'open'
  const first = agent.fitHeight(100)
  await flushPromises()
  const intermediate = agent.fitHeight(200)
  const latest = agent.fitHeight(300)
  expect(heights).toEqual([100])
  finish!()
  await flushPromises()
  expect(heights).toEqual([100, 300])
  finish!()
  await Promise.all([first, intermediate, latest])
  await agent.fitHeight(300)
  expect(heights).toEqual([100, 300])
  wrapper.unmount()
})