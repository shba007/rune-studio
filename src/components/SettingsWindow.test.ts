import RuneNavItem from './RuneNavItem.vue'
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
  it('orders the five navigation pages, and removes the saved footer', () => {
    const wrapper = shallowMount(SettingsWindow, { global: { stubs: { RuneNavItem } } })
    expect(wrapper.findAll('nav button').map((button) => button.text())).toEqual([
      'General',
      'Models & tools',
      'Capture & privacy',
      'Developer',
      'About',
    ])
    expect(wrapper.find('[data-testid="saved"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('has no section search field', () => {
    const wrapper = shallowMount(SettingsWindow, { global: { stubs: { RuneNavItem } } })
    expect(wrapper.find('input[type="search"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('separates the system sections with one divider', () => {
    const wrapper = shallowMount(SettingsWindow, { global: { stubs: { RuneNavItem } } })
    const nav = wrapper.get('nav')
    const dividers = nav.findAll('[data-testid="nav-divider"]')
    expect(dividers).toHaveLength(1)
    const order = nav.findAll('button, [data-testid="nav-divider"]').map((node) => node.text())
    expect(order.indexOf('')).toBe(order.indexOf('Developer') - 1)
    wrapper.unmount()
  })
  it('opens task history in the workspace rather than a Settings section, including old links', async () => {
    window.history.replaceState({}, '', '/?section=artifacts')
    const wrapper = shallowMount(SettingsWindow, { global: { stubs: { RuneNavItem } } })
    const agent = vi.mocked(useAgent).mock.results.at(-1)!.value
    expect(agent.openWorkspace).toHaveBeenCalledWith('tasks')
    expect(wrapper.find('[data-testid="nav-artifacts"]').exists()).toBe(false)
    expect(wrapper.findAll('nav button').some((button) => button.text() === 'Workspace')).toBe(
      false,
    )
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
    const wrapper = shallowMount(SettingsWindow, { global: { stubs: { RuneNavItem } } })
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
    const wrapper = shallowMount(SettingsWindow, { global: { stubs: { RuneNavItem } } })
    await flushPromises()
    expect(wrapper.find('[data-testid="nav-life"]').exists()).toBe(false)
    expect(wrapper.find('h1').text()).toBe('General')
    expect(wrapper.findComponent({ name: 'SettingsLife' }).exists()).toBe(false)
    wrapper.unmount()
  })
})