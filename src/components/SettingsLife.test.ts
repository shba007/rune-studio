import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'

import type { Settings } from '@/types/protocol'
import SettingsLife from './SettingsLife.vue'

function makeDraft(overrides: Partial<Settings> = {}): Settings {
  return {
    verifyMode: 'background',
    demoMode: true,
    recordSessionVideo: 'off',
    modelImageResolution: '1280',
    recordingResolution: 'native',
    offerReview: 'always',
    reflectionEnabled: true,
    idleMinutes: 10,
    archiveKeepDays: 0,
    baseUrl: 'http://127.0.0.1:8080',
    model: 'gemma-4-12b-it',
    models: {
      agent: 'gemma-4-12b-it',
      verifier: 'gemma-4-12b-it',
      analysis: 'gemma-4-12b-it',
      voice: 'gemma-4-12b-it',
      decision: 'liquid-d1-3b',
      advisor: 'gemma-4-12b-it',
    },
    maxSteps: 30,
    toggleHotkey: 'Ctrl+Shift+Space',
    stopHotkey: 'Ctrl+Alt+Esc',
    externalAgent: false,
    externalAgentSince: 0,
    showThinking: false,
    askEveryAction: false,
    developerMode: false,
    fastChecks: false,
    decisionModel: 'liquid-d1-3b',
    decisionUrl: '',
    lifeEnabled: false,
    lifeConsents: {
      general: false,
      health: false,
      mind: false,
      finance: false,
    },
    retentionDays: {
      general: 0,
      health: 0,
      mind: 0,
      finance: 0,
    },
    ...overrides,
  }
}

describe('SettingsLife', () => {
  it('renders privacy disclaimer marked OWNER TO REVIEW', () => {
    const draft = makeDraft()
    const wrapper = mount(SettingsLife, { props: { draft } })

    expect(wrapper.text()).toContain('OWNER TO REVIEW')
    expect(wrapper.text()).toContain('Rune is not a clinician')
  })

  it('hides domain classes when master switch is off', () => {
    const draft = makeDraft({ lifeEnabled: false })
    const wrapper = mount(SettingsLife, { props: { draft } })

    expect(wrapper.find('[data-testid="class-row-health"]').exists()).toBe(false)
  })

  it('shows domain classes and toggles consent when enabled', async () => {
    const draft = makeDraft({ lifeEnabled: true })
    const wrapper = mount(SettingsLife, { props: { draft } })

    expect(wrapper.find('[data-testid="class-row-general"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="class-row-health"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="class-row-mind"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="class-row-finance"]').exists()).toBe(true)

    const healthConsent = wrapper.find<HTMLInputElement>('[data-testid="consent-health"]')
    expect(healthConsent.element.checked).toBe(false)

    await healthConsent.setValue(true)
    expect(draft.lifeConsents?.health).toBe(true)
  })

  it('prompts confirmation when delete is clicked and requires typing class name', async () => {
    const draft = makeDraft({ lifeEnabled: true })
    const wrapper = mount(SettingsLife, { props: { draft } })

    await wrapper.find('[data-testid="delete-finance"]').trigger('click')
    expect(wrapper.find('[data-testid="delete-confirm-box"]').exists()).toBe(true)

    const confirmInput = wrapper.find<HTMLInputElement>('[data-testid="delete-confirm-input"]')
    await confirmInput.setValue('wrong')
    await wrapper.find('[data-testid="delete-confirm-btn"]').trigger('click')
    expect(wrapper.text()).toContain('Type "finance" to confirm deletion')

    await confirmInput.setValue('finance')
    await wrapper.find('[data-testid="delete-confirm-btn"]').trigger('click')
    expect(wrapper.text()).toContain('permanently purged and vacuumed')
    expect(wrapper.find('[data-testid="delete-confirm-box"]').exists()).toBe(false)
  })
})