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
    () =>
      ({
        settings: ref(null),
        notice: ref(null),
        bridge: ref(null),
        openWorkspace: vi.fn<() => void>(),
      }) as unknown as Agent,
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
      'Connections',
      'Agents',
      'Tools & connections',
      'Capture & recording',
      'Data & privacy',
      'Shortcuts',
      'Developer',
      'About',
      'Knowledge',
    ])
    expect(wrapper.find('[data-testid="saved"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('searches applicable sections by settings keywords and retains Knowledge access', async () => {
    const wrapper = shallowMount(SettingsWindow)
    await wrapper.get('input[aria-label="Search settings"]').setValue('unmatched-setting')
    expect(wrapper.text()).toContain('No matching sections')
    await wrapper.get('input[aria-label="Search settings"]').setValue('voice')
    expect(wrapper.findAll('nav button').map((button) => button.text())).toEqual([
      'Connections',
      'Knowledge',
    ])
    await wrapper.get('[data-testid="nav-models"]').trigger('click')
    expect(wrapper.get('h1').text()).toBe('Connections')
    await wrapper.get('input[aria-label="Search settings"]').setValue('')
    expect(wrapper.find('[data-testid="nav-capture"]').exists()).toBe(true)
    wrapper.unmount()
  })
  it('opens task history in the workspace rather than a Settings section, including old links', async () => {
    window.history.replaceState({}, '', '/?section=artifacts')
    const wrapper = shallowMount(SettingsWindow)
    const agent = vi.mocked(useAgent).mock.results.at(-1)!.value
    expect(agent.openWorkspace).toHaveBeenCalledWith('tasks')
    expect(wrapper.find('[data-testid="nav-artifacts"]').exists()).toBe(false)
    await wrapper.findAll('nav button').at(-1)!.trigger('click')
    expect(agent.openWorkspace).toHaveBeenCalledTimes(2)
    wrapper.unmount()
  })
  it('saves nested Analysis model edits without mutating the saved settings', async () => {
    vi.useFakeTimers()
    const settings = ref({
      models: { agent: 'vision', analysis: '' },
      lifeConsents: { health: false },
    } as unknown as Settings)
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
    draft.lifeConsents!.health = true
    await wrapper.vm.$nextTick()
    expect(settings.value.models.analysis).toBe('')
    expect(settings.value.lifeConsents!.health).toBe(false)
    await vi.advanceTimersByTimeAsync(650)
    expect(saveSettings).toHaveBeenCalledOnce()
    expect(settings.value.models.analysis).toBe('vision')
    expect(settings.value.lifeConsents!.health).toBe(true)
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