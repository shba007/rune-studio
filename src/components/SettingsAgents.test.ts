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
    const wrapper = mount(SettingsAgents, { props: { agent, draft } })
    const select = wrapper.get('select')
    expect(wrapper.text()).toContain('API')
    await select.setValue('true')
    expect(draft.externalAgent).toBe(true)
    expect(wrapper.text()).toContain('without calling its API model')
    running.value = true
    await wrapper.vm.$nextTick()
    expect(select.attributes('disabled')).toBeDefined()
    running.value = false
    await wrapper.vm.$nextTick()
    await select.setValue('false')
    expect(draft.externalAgent).toBe(false)
    wrapper.unmount()
  })
})