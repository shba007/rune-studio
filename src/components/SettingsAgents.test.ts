import { mount } from '@vue/test-utils'
import { reactive, ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import SettingsAgents from './SettingsAgents.vue'
import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

describe('execution mode', () => {
  it('selects one flow before a session and disables switching during execution', async () => {
    const draft = reactive({ externalAgent: false }) as Settings
    const running = ref(false)
    const agent = { isRunning: running, bridge: ref(null) } as unknown as Agent
    const wrapper = mount(SettingsAgents, {
      props: { agent, draft },
      global: { stubs: { SettingsConnectionModels: true } },
    })
    const choice = (name: string) =>
      wrapper.findAll('[role="radio"]').find((button) => button.text() === name)!
    expect(choice('Studio model').attributes('aria-checked')).toBe('true')
    await choice('External agent').trigger('click')
    expect(draft.externalAgent).toBe(true)
    await wrapper.get('button[aria-label^="Details"]').trigger('click')
    expect(wrapper.text()).toContain('without calling its API model')
    running.value = true
    await wrapper.vm.$nextTick()
    expect(choice('Studio model').attributes('disabled')).toBeDefined()
    running.value = false
    await wrapper.vm.$nextTick()
    await choice('Studio model').trigger('click')
    expect(draft.externalAgent).toBe(false)
    wrapper.unmount()
  })
})