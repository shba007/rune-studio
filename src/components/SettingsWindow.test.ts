import { flushPromises, shallowMount } from '@vue/test-utils'
import { ref } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vite-plus/test'
import SettingsWindow from './SettingsWindow.vue'

vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))
vi.mock('@/composables/useAgent', () => ({
  useAgent: () => ({ settings: ref(null), notice: ref(null), bridge: ref(null) }),
}))

afterEach(() => window.history.replaceState({}, '', '/'))

describe('release settings availability', () => {
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