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
  it('requires an explicit Analysis model instead of advertising an agent fallback', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Screen Analysis',
        roleKey: 'analysis',
        agentModel: 'local-omni',
        modelValue: '',
        models: fakeModels,
      },
    })
    expect(wrapper.text()).not.toContain('Using agent model')
    expect(wrapper.text()).toContain('Choose an explicit Analysis model')
    expect(wrapper.find('input').attributes('placeholder')).not.toBe('Same as agent')
  })

  it('warns when Analysis capabilities are unknown or tool calling is unavailable', () => {
    for (const toolCalls of ['no', 'unknown'] as const) {
      const wrapper = mount(ModelRoleRow, {
        props: {
          roleLabel: 'Screen Analysis',
          roleKey: 'analysis',
          agentModel: 'local-omni',
          modelValue: 'local-omni',
          models: [{ ...fakeModels[0]!, toolCalls }],
        },
      })
      expect(wrapper.text()).toContain('advertises image input and tool calling')
    }
  })

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
    expect(wrapper.text()).toContain('Cloud model')
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
    expect(wrapper.text()).not.toContain('Cloud model')
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
    expect(wrapper.text()).toContain('Model capabilities')
  })
})

describe('Model settings clarity', () => {
  it('names capabilities and distinguishes unsupported from unknown', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Agent',
        roleKey: 'agent',
        agentModel: '',
        modelValue: 'cloud-text-only',
        models: fakeModels,
      },
    })
    expect(wrapper.get('label').attributes('for')).toBe(wrapper.get('input').attributes('id'))
    expect(wrapper.text()).toContain('See images')
    expect(wrapper.text()).toContain('Not supported')
    expect(wrapper.text()).toContain('Unknown')
    expect(wrapper.text()).not.toContain('Caps:')
  })

  it('explains an undiscovered model instead of implying support', () => {
    const wrapper = mount(ModelRoleRow, {
      props: {
        roleLabel: 'Voice',
        roleKey: 'voice',
        agentModel: 'not-discovered',
        modelValue: '',
        models: fakeModels,
      },
    })
    expect(wrapper.text()).toContain('Using agent model · not-discovered')
    expect(wrapper.text()).toContain('Capabilities unknown')
    expect(wrapper.find('details').exists()).toBe(false)
  })
})