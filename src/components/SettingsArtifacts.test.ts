import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import SettingsArtifacts from './SettingsArtifacts.vue'
import type { Agent } from '@/composables/useAgent'

const invoke = vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, args?: Record<string, unknown>) => invoke(command, args),
  convertFileSrc: (path: string) => path,
}))
vi.mock('@vueuse/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@vueuse/core')>()),
  useIntersectionObserver: () => ({ stop: vi.fn<() => void>() }),
}))
const rows = ['First session', 'Second session'].map((task, index) => ({
  id: String(index),
  task,
  source: 'built-in',
  outcome: 'succeeded',
  startedUtc: null,
  endedUtc: '2026-10-26T00:00:00Z',
  steps: 1,
  thumb: null,
  verdict: null,
}))
const openReview = vi.fn<(id: string) => void>()
beforeEach(() => {
  invoke.mockReset()
  openReview.mockReset()
})

async function show(failure = false) {
  invoke.mockImplementation(async (command) => {
    if (command === 'flight_runs') return rows
    if (command === 'flight_delete' && failure) throw 'Deletion refused: finish this session first'
    return null
  })
  const wrapper = mount(SettingsArtifacts, { props: { agent: { openReview } as unknown as Agent } })
  await flushPromises()
  return wrapper
}

describe('session deletion', () => {
  it('removes only the selected card after deletion succeeds', async () => {
    const wrapper = await show()
    await wrapper.findAll('button[aria-label="Delete session"]')[0]!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('flight_delete', { id: '0' })
    expect(wrapper.findAll('[data-testid="artifact"]')).toHaveLength(1)
    expect(wrapper.text()).toContain('Second session')
    expect(openReview).not.toHaveBeenCalled()
  })

  it('keeps the cards and reports a failed deletion', async () => {
    const wrapper = await show(true)
    await wrapper.findAll('button[aria-label="Delete session"]')[0]!.trigger('click')
    await flushPromises()
    expect(wrapper.findAll('[data-testid="artifact"]')).toHaveLength(2)
    expect(wrapper.get('[role="alert"]').text()).toContain('Deletion refused')
  })
})