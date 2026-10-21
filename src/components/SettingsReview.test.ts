import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vite-plus/test'
import type { Settings } from '@/types/protocol'
import SettingsReview from './SettingsReview.vue'

describe('session review settings', () => {
  it('reflects saved preferences and updates every choice', async () => {
    const draft = { offerReview: 'failures', recordSessionVideo: 'motion' } as Settings
    const wrapper = mount(SettingsReview, { props: { draft } })
    const review = wrapper.get<HTMLSelectElement>('[aria-label="Offer session review"]')
    const video = wrapper.get<HTMLSelectElement>('[aria-label="Session video"]')
    expect(review.element.value).toBe('failures')
    expect(video.element.value).toBe('motion')
    for (const choice of ['always', 'never', 'failures']) {
      await review.setValue(choice)
      expect(draft.offerReview).toBe(choice)
    }
    for (const choice of ['off', 'motion']) {
      await video.setValue(choice)
      expect(draft.recordSessionVideo).toBe(choice)
    }
    expect(wrapper.text()).toContain('2 frames per second')
    expect(wrapper.text()).toContain('cost is unmeasured')
  })
})