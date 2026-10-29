import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { invoke } from '@tauri-apps/api/core'
import SettingsTools from './SettingsTools.vue'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn<typeof invoke>() }))
beforeEach(() => vi.mocked(invoke).mockReset())
describe('Kit settings', () => {
  it('saves the HTTP endpoint and explains server-owned filesystem confinement', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        enabled: true,
        endpoint: 'http://127.0.0.1:47601/mcp',
        selected: [],
      })
      .mockResolvedValue(undefined)
    const wrapper = mount(SettingsTools)
    await flushPromises()
    const root = wrapper.get('input[placeholder="http://127.0.0.1:47601/mcp"]')
    await root.setValue('http://127.0.0.1:47602/mcp')
    await root.trigger('change')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('kit_save', {
      config: { enabled: true, endpoint: 'http://127.0.0.1:47602/mcp', selected: [] },
    })
    expect(wrapper.text()).toContain('File contents are sent to the selected model')
  })
  it('keeps tools off and unselected, surfaces discovery errors without treating them as empty success', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        enabled: false,
        endpoint: 'http://127.0.0.1:47601/mcp',
        selected: [],
      })
      .mockRejectedValueOnce('Git plugin discovery failed')
    const wrapper = mount(SettingsTools)
    await flushPromises()
    expect(wrapper.get('button[role=switch]').attributes('aria-checked')).toBe('false')
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Refresh / discover')!
      .trigger('click')
    await flushPromises()
    expect(wrapper.get('[role=status]').text()).toContain('Git plugin discovery failed')
    expect(invoke).toHaveBeenCalledTimes(2)
  })
  it('shows the known model limitation and prevents enabling tools', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      enabled: false,
      endpoint: 'http://127.0.0.1:47601/mcp',
      selected: [],
    })
    const wrapper = mount(SettingsTools, { props: { toolCalls: 'no' } })
    await flushPromises()
    expect(wrapper.get('[role=alert]').text()).toContain('does not support tool calling')
    expect(wrapper.get('button[role=switch]').attributes('disabled')).toBeDefined()
  })
})