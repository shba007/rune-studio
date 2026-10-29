import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
import AppIcon from './AppIcon.vue'
import { shallowMount } from '@vue/test-utils'
import { reactive, ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import SettingsGeneral from './SettingsGeneral.vue'
import DevInputLab from './DevInputLab.vue'
import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

describe('developer controls', () => {
  it('keeps model and capture resolution controls independent', async () => {
    const draft = reactive({
      modelImageResolution: '1280',
      recordingResolution: 'native',
      developerMode: false,
    }) as Settings
    const wrapper = shallowMount(SettingsGeneral, {
      props: { agent: {} as Agent, draft },
      global: {
        stubs: {
          RuneChoice,
          AppIcon,
          HubGroup: { template: '<div><slot /></div>' },
          HubRow: { template: '<div><slot /></div>' },
        },
      },
    })
    await wrapper.get('button[aria-label="Model input resolution"]').trigger('click')
    await wrapper
      .findAll('[role="listbox"] [role="option"]')
      .find((option) => option.text().startsWith('1920'))!
      .trigger('click')
    expect(draft.modelImageResolution).toBe('1920')
    expect(draft.recordingResolution).toBe('native')
    await wrapper.get('button[aria-label="Screenshot and recording resolution"]').trigger('click')
    await wrapper
      .findAll('[role="listbox"] [role="option"]')
      .find((option) => option.text().startsWith('720'))!
      .trigger('click')
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
    const wrapper = shallowMount(SettingsGeneral, {
      props: { agent, draft, apiKey: '' },
      global: {
        stubs: {
          RuneButton,
          RuneChoice,
          AppIcon,
          HubGroup: { template: '<div><slot /></div>' },
          HubRow: { template: '<div><slot /></div>' },
        },
      },
    })
    expect(wrapper.find('[role="listbox"]').exists()).toBe(false)
    expect(wrapper.find('button[aria-label="Preview state"]').exists()).toBe(false)
    expect(wrapper.findComponent(DevInputLab).exists()).toBe(false)
    draft.developerMode = true
    await wrapper.vm.$nextTick()
    expect(wrapper.find('button[aria-label="Preview state"]').exists()).toBe(true)
    expect(wrapper.findComponent(DevInputLab).exists()).toBe(true)
    await wrapper.get('button[aria-label="Preview state"]').trigger('click')
    await wrapper
      .findAll('[role="option"]')
      .find((option) => option.text() === 'acting')!
      .trigger('click')
    expect(debugSetState).toHaveBeenCalledWith('acting')
    draft.developerMode = false
    await wrapper.vm.$nextTick()
    expect(wrapper.findComponent(DevInputLab).exists()).toBe(false)
    wrapper.unmount()
  })
})