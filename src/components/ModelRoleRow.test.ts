import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'

import type { ModelCapabilities } from '@/types/protocol'
import ModelRoleRow from './ModelRoleRow.vue'

const fakeModels: ModelCapabilities[] = [
  {
    id: 'local-omni',
    modalitiesIn: ['text', 'image', 'audio'],
    modalitiesOut: ['text'],
    audioIn: 'yes',
    imageIn: 'yes',
    speechOut: 'no',
    toolCalls: 'yes',
    thinkingControl: 'yes',
    maxImageEdgePx: 1280,
    contextTokens: 131072,
    locality: 'local',
    resident: 'yes',
    vramMbEstimate: 8000,
    loadMsEstimate: 2000,
    status: 'ready',
    rawCapabilities: ['Chat', 'Vision', 'Audio', 'Tool Call'],
  },
  {
    id: 'cloud-text-only',
    modalitiesIn: ['text'],
    modalitiesOut: ['text'],
    audioIn: 'no',
    imageIn: 'no',
    speechOut: 'no',
    toolCalls: 'no',
    thinkingControl: 'unknown',
    maxImageEdgePx: null,
    contextTokens: 32768,
    locality: 'cloud',
    resident: 'unknown',
    vramMbEstimate: null,
    loadMsEstimate: null,
    status: null,
    rawCapabilities: ['Chat'],
  },
]

describe('ModelRoleRow', () => {
  it('shows CLOUD tag for a cloud model', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Agent',
        roleKey: 'agent',
        agentModel: 'local-omni',
        modelValue: 'cloud-text-only',
        models: fakeModels,
      },
    })
    expect(wrapper.text()).toContain('CLOUD')
  })

  it('does not show CLOUD tag for a local model', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Agent',
        roleKey: 'agent',
        agentModel: 'local-omni',
        modelValue: 'local-omni',
        models: fakeModels,
      },
    })
    expect(wrapper.text()).not.toContain('CLOUD')
  })

  it('warns when voice role has a model lacking audio input', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Voice',
        roleKey: 'voice',
        agentModel: 'local-omni',
        modelValue: 'cloud-text-only',
        models: fakeModels,
      },
    })
    expect(wrapper.text()).toContain('does not support audio input')
  })

  it('warns when agent role lacks image input or tool calling', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Agent',
        roleKey: 'agent',
        agentModel: 'cloud-text-only',
        modelValue: 'cloud-text-only',
        models: fakeModels,
      },
    })
    expect(wrapper.text()).toContain('lacks image input')
  })

  it('falls back to agent model when role model is empty and displays agent caps', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Verifier',
        roleKey: 'verifier',
        agentModel: 'local-omni',
        modelValue: '',
        models: fakeModels,
      },
    })
    // Since fallback is local-omni (which has imageIn yes), no warning should appear
    expect(wrapper.text()).not.toContain('lacks image input')
    // Caps should show for local-omni
    expect(wrapper.text()).toContain('Caps:')
  })
})