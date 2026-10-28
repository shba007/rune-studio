import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import OrbDock from './OrbDock.vue'
import type { Agent } from '@/composables/useAgent'

describe('Goals button routing', () => {
  it('opens the Workspace even while a task is running in either execution mode', async () => {
    const settings = ref({ externalAgent: true })
    const running = ref(true)
    const openWorkspace = vi.fn<(section: string) => void>()
    const agent = { settings, isRunning: running, openWorkspace } as unknown as Agent
    const wrapper = mount(OrbDock, { props: { agent } })
    const button = wrapper.get('button[aria-label="Workspace"]')
    expect(button.attributes('disabled')).toBeUndefined()
    await button.trigger('click')
    expect(openWorkspace).toHaveBeenCalledWith('overview')
    expect(wrapper.find('button[aria-label="Visual Analysis"]').exists()).toBe(false)
    settings.value.externalAgent = false
    await wrapper.vm.$nextTick()
    expect(button.attributes('disabled')).toBeUndefined()
    running.value = false
    await wrapper.vm.$nextTick()
    expect(button.attributes('disabled')).toBeUndefined()
    wrapper.unmount()
  })
})