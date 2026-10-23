import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { invoke } from '@tauri-apps/api/core'
import SettingsTools from './SettingsTools.vue'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn<typeof invoke>() }))
beforeEach(() => vi.mocked(invoke).mockReset())
describe('Kit settings', () => {
  it('keeps tools off and unselected, surfaces discovery errors without treating them as empty success', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ enabled: false, executable: '', selected: [] })
      .mockRejectedValueOnce('Git plugin discovery failed')
    const wrapper = mount(SettingsTools)
    await flushPromises()
    expect(wrapper.get<HTMLInputElement>('input[type=checkbox]').element.checked).toBe(false)
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role=status]').text()).toContain('Git plugin discovery failed')
    expect(invoke).toHaveBeenCalledTimes(2)
  })
  it('shows the known model limitation and prevents enabling tools', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ enabled: false, executable: '', selected: [] })
    const wrapper = mount(SettingsTools, { props: { toolCalls: 'no' } })
    await flushPromises()
    expect(wrapper.get('[role=alert]').text()).toContain('does not support tool calling')
    expect(wrapper.get<HTMLInputElement>('input[type=checkbox]').element.disabled).toBe(true)
  })
})