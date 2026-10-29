import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'
import type { Settings } from '@/types/protocol'
import SettingsReview from './SettingsReview.vue'

describe('session review settings', () => {
  it('reflects saved preferences and updates every choice', async () => {
    const draft = { offerReview: 'failures', recordSessionVideo: 'motion' } as Settings
    const wrapper = mount(SettingsReview, { props: { draft } })
    const pick = async (group: string, name: string) =>
      wrapper
        .get(`[aria-label="${group}"]`)
        .findAll('[role="radio"]')
        .find((button) => button.text() === name)!
        .trigger('click')
    const checked = (group: string) =>
      wrapper
        .get(`[aria-label="${group}"]`)
        .findAll('[role="radio"]')
        .find((button) => button.attributes('aria-checked') === 'true')!
        .text()
    expect(checked('Offer task review')).toBe('Failures')
    expect(checked('Task video')).toBe('Motion')
    for (const [name, choice] of [
      ['Always', 'always'],
      ['Never', 'never'],
      ['Failures', 'failures'],
    ]) {
      await pick('Offer task review', name!)
      expect(draft.offerReview).toBe(choice)
    }
    for (const [name, choice] of [
      ['Off', 'off'],
      ['Motion', 'motion'],
    ]) {
      await pick('Task video', name!)
      expect(draft.recordSessionVideo).toBe(choice)
    }
    expect(wrapper.text()).toContain('2 frames per second')
    expect(wrapper.text()).toContain('cost is unmeasured')
  })
})