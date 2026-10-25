import { shallowMount } from '@vue/test-utils'
import { reactive, ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import SettingsGeneral from './SettingsGeneral.vue'
import DevInputLab from './DevInputLab.vue'
import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

describe('developer controls', () => {
  it('keeps model and recording resolution controls independent', async () => {
    const draft = reactive({
      modelImageResolution: '1280',
      recordingResolution: 'native',
      developerMode: false,
    }) as Settings
    const wrapper = shallowMount(SettingsGeneral, {
      props: { agent: {} as Agent, draft },
      global: {
        stubs: {
          HubGroup: { template: '<div><slot /></div>' },
          HubRow: { template: '<div><slot /></div>' },
        },
      },
    })
    await wrapper.get('select[aria-label="Model image resolution"]').setValue('1920')
    expect(draft.modelImageResolution).toBe('1920')
    expect(draft.recordingResolution).toBe('native')
    await wrapper.get('select[aria-label="Recording resolution"]').setValue('720')
    expect(draft.recordingResolution).toBe('720')
    expect(draft.modelImageResolution).toBe('1920')
    wrapper.unmount()
  })

  it('uses the saved developer setting rather than the development build flag', async () => {
    const draft = reactive({
      developerMode: false,
      models: {},
      model: '',
      decisionModel: '',
    }) as Settings
    const debugSetState = vi.fn<(state: string) => void>()
    const agent = { backend: ref(null), debugSetState } as unknown as Agent
    const wrapper = shallowMount(SettingsGeneral, { props: { agent, draft, apiKey: '' } })
    expect(wrapper.find('details').exists()).toBe(false)
    expect(wrapper.findComponent(DevInputLab).exists()).toBe(false)
    draft.developerMode = true
    await wrapper.vm.$nextTick()
    expect(wrapper.find('details').exists()).toBe(true)
    expect(wrapper.findComponent(DevInputLab).exists()).toBe(true)
    const acting = wrapper.findAll('details button').find((button) => button.text() === 'acting')!
    await acting.trigger('click')
    expect(debugSetState).toHaveBeenCalledWith('acting')
    draft.developerMode = false
    await wrapper.vm.$nextTick()
    expect(wrapper.findComponent(DevInputLab).exists()).toBe(false)
    wrapper.unmount()
  })
})