import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import OrbDock from './OrbDock.vue'
import type { Agent } from '@/composables/useAgent'

describe('Screen Analysis button routing', () => {
  it('allows external requests during a session while keeping API captures blocked during a task', async () => {
    const settings = ref({ externalAgent: true })
    const running = ref(true)
    const analyzeScreen = vi.fn<() => void>()
    const agent = { settings, isRunning: running, analyzeScreen } as unknown as Agent
    const wrapper = mount(OrbDock, { props: { agent } })
    const button = wrapper.get('button[aria-label="Analyse the screen"]')
    expect(button.attributes('disabled')).toBeUndefined()
    expect(button.attributes('title')).toContain('external agent')
    await button.trigger('click')
    expect(analyzeScreen).toHaveBeenCalledOnce()
    settings.value.externalAgent = false
    await wrapper.vm.$nextTick()
    expect(button.attributes('disabled')).toBeDefined()
    running.value = false
    await wrapper.vm.$nextTick()
    expect(button.attributes('disabled')).toBeUndefined()
    wrapper.unmount()
  })
})