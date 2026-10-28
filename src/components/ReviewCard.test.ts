import { mount } from '@vue/test-utils'
import { nextTick, ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'

import type { Agent } from '@/composables/useAgent'
import type { ReviewOffer } from '@/config/review'

import ReviewCard from './ReviewCard.vue'

function fake(offer: (ReviewOffer & { endsAt: number }) | null) {
  const reviewOffer = ref(offer)
  const dismissReview = vi.fn<() => void>(() => (reviewOffer.value = null))
  const openReview = vi.fn<(runId: string) => void>()
  return {
    agent: { reviewOffer, dismissReview, openReview } as unknown as Agent,
    dismissReview,
    openReview,
  }
}

const offer = (over: Partial<ReviewOffer> = {}, endsIn = 15_000) => ({
  runId: 'r1',
  task: 'Open Notepad and write hello',
  outcome: 'succeeded' as const,
  steps: 6,
  durationSecs: 55,
  source: 'Rune',
  endsAt: Date.now() + endsIn,
  ...over,
})

describe('ReviewCard', () => {
  it('shows the task, the outcome chip, the time and the steps, and two buttons', () => {
    const { agent } = fake(offer())
    const wrapper = mount(ReviewCard, { props: { agent } })
    expect(wrapper.text()).toContain('Open Notepad and write hello')
    expect(wrapper.find('[data-testid="outcome"]').text()).toBe('succeeded')
    expect(wrapper.find('[data-testid="meta"]').text()).toBe('0:55 · 6 STEPS')
    expect(wrapper.findAll('button').map((b) => b.text())).toEqual(['Dismiss', 'Review'])
  })

  it('marks a failure in red and nothing else', () => {
    const failed = mount(ReviewCard, { props: { agent: fake(offer({ outcome: 'failed' })).agent } })
    const ok = mount(ReviewCard, { props: { agent: fake(offer()).agent } })
    expect(failed.find('[data-testid="outcome"]').classes().join(' ')).toContain('attention')
    expect(ok.html()).not.toContain('attention')
  })

  it('Review opens the session and Dismiss puts the card away', async () => {
    const { agent, openReview, dismissReview } = fake(offer())
    const wrapper = mount(ReviewCard, { props: { agent } })
    await wrapper.findAll('button')[1]!.trigger('click')
    expect(openReview).toHaveBeenCalledWith('r1')
    await wrapper.findAll('button')[0]!.trigger('click')
    expect(dismissReview).toHaveBeenCalled()
  })

  it('shows the countdown as dots only, all lit at the start', () => {
    const { agent } = fake(offer())
    const wrapper = mount(ReviewCard, { props: { agent } })
    const dots = wrapper.find('[data-testid="dots"]').findAll('span')
    expect(dots).toHaveLength(15)
    expect(
      dots.filter((dot) => dot.classes().includes('opacity-80')).length,
    ).toBeGreaterThanOrEqual(14)
    expect(wrapper.text()).not.toMatch(/\d+ ?s\b/)
  })

  it('goes by itself when the time is up', async () => {
    const { agent, dismissReview } = fake(offer({}, -1_000))
    mount(ReviewCard, { props: { agent } })
    await nextTick()
    await new Promise((resolve) => setTimeout(resolve, 300))
    expect(dismissReview).toHaveBeenCalled()
  })

  it('renders nothing without an offer', () => {
    const wrapper = mount(ReviewCard, { props: { agent: fake(null).agent } })
    expect(wrapper.find('section').exists()).toBe(false)
  })
})