import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import ArtifactCard from './ArtifactCard.vue'
import type { FlightRun } from '@/config/flight'

const invoke = vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>()
const writeText = vi.fn<(text: string) => Promise<void>>()
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, args?: Record<string, unknown>) => invoke(command, args),
  convertFileSrc: (path: string) => path,
}))
vi.mock('@vueuse/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@vueuse/core')>()),
  useIntersectionObserver: () => ({ stop: vi.fn<() => void>() }),
}))
const run: FlightRun = {
  id: 'one',
  task: 'A cleaned-up task title',
  source: 'built-in',
  outcome: 'succeeded',
  startedUtc: null,
  endedUtc: '2026-10-26T00:00:00Z',
  steps: 1,
  verdict: null,
  thumb: null,
}

beforeEach(() => {
  invoke.mockReset()
  writeText.mockReset().mockResolvedValue()
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
})

describe('session card actions', () => {
  it('copies the exact retained input without opening the replay', async () => {
    const original = '  open notepad\nand type বাংলা  '
    invoke.mockResolvedValue(original)
    const wrapper = mount(ArtifactCard, { props: { run } })
    await wrapper.get('button[aria-label="Copy original prompt"]').trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('flight_original_prompt', { id: 'one' })
    expect(writeText).toHaveBeenCalledWith(original)
    expect(wrapper.emitted('open')).toBeUndefined()
    expect(wrapper.text()).toContain('Prompt copied')
    expect(wrapper.find('button button').exists()).toBe(false)
  })

  it('does not copy a generated title when the original is unavailable', async () => {
    invoke.mockRejectedValue('The original prompt was not retained.')
    const wrapper = mount(ArtifactCard, { props: { run } })
    await wrapper.get('button[aria-label="Copy original prompt"]').trigger('click')
    await flushPromises()
    expect(writeText).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('not retained')
  })

  it('deletes only through its dedicated action and protects active sessions', async () => {
    const wrapper = mount(ArtifactCard, { props: { run } })
    await wrapper.get('button[aria-label="Delete task"]').trigger('click')
    expect(wrapper.emitted('remove')).toHaveLength(1)
    expect(wrapper.emitted('open')).toBeUndefined()
    await wrapper.setProps({ run: { ...run, active: true } })
    expect(wrapper.get('button[aria-label="Delete task"]').attributes('disabled')).toBeDefined()
    await wrapper.get('button[aria-label="Delete task"]').trigger('click')
    expect(wrapper.emitted('remove')).toHaveLength(1)
  })
})