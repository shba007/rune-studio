import { flushPromises, shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'
import SettingsWindow from './SettingsWindow.vue'
import SettingsGeneral from './SettingsGeneral.vue'
import { useAgent } from '@/composables/useAgent'
import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))
vi.mock('@/composables/useAgent', () => ({
  useAgent: vi.fn<() => Agent>(
    () => ({ settings: ref(null), notice: ref(null), bridge: ref(null) }) as unknown as Agent,
  ),
}))

afterEach(() => {
  vi.useRealTimers()
  window.history.replaceState({}, '', '/')
})

describe('release settings availability', () => {
  it('keeps Agents available without the bridge, orders navigation, and removes the saved footer', () => {
    const wrapper = shallowMount(SettingsWindow)
    expect(wrapper.findAll('nav button').map((button) => button.text())).toEqual([
      'General',
      'Agents',
      'Tools',
      'Tasks',
      'Shortcuts',
      'About',
    ])
    expect(wrapper.find('[data-testid="saved"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('saves nested Analysis model edits without mutating the saved settings', async () => {
    vi.useFakeTimers()
    const settings = ref({ models: { agent: 'vision', analysis: '' } } as Settings)
    const saveSettings = vi.fn<(next: Settings) => Promise<void>>(async (next) => {
      settings.value = JSON.parse(JSON.stringify(next)) as Settings
    })
    vi.mocked(useAgent).mockReturnValueOnce({
      settings,
      saveSettings,
      notice: ref(null),
      bridge: ref(null),
    } as unknown as Agent)
    const wrapper = shallowMount(SettingsWindow)
    const draft = wrapper.getComponent(SettingsGeneral).props('draft') as Settings
    draft.models.analysis = 'vision'
    await wrapper.vm.$nextTick()
    expect(settings.value.models.analysis).toBe('')
    await vi.advanceTimersByTimeAsync(650)
    expect(saveSettings).toHaveBeenCalledOnce()
    expect(settings.value.models.analysis).toBe('vision')
    wrapper.unmount()
    vi.useRealTimers()
  })

  it('does not expose unfinished life controls, including through an old deep link', async () => {
    window.history.replaceState({}, '', '/?section=life')
    const wrapper = shallowMount(SettingsWindow)
    await flushPromises()
    expect(wrapper.find('[data-testid="nav-life"]').exists()).toBe(false)
    expect(wrapper.find('h1').text()).toBe('General')
    expect(wrapper.findComponent({ name: 'SettingsLife' }).exists()).toBe(false)
    wrapper.unmount()
  })
})