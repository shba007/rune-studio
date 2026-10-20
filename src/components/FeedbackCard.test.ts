import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vite-plus/test'
import { ref } from 'vue'

import type { Agent } from '@/composables/useAgent'

import FeedbackCard from './FeedbackCard.vue'

function fakeAgent(stage: 'complete' | 'why') {
  const answerComplete = vi.fn<(complete: boolean | null) => void>()
  const answerWhy = vi.fn<(reason: string) => void>()
  const agent = {
    reviewOffer: ref({
      runId: 'r1',
      task: 'open notepad',
      outcome: 'succeeded' as const,
      steps: 3,
      durationSecs: 20,
      source: 'Rune',
      endsAt: 0,
    }),
    reviewStage: ref(stage),
    answerComplete,
    answerWhy,
  }
  return { agent: agent as unknown as Agent, answerComplete, answerWhy }
}

describe('FeedbackCard', () => {
  it('asks whether the task was completed and passes the answer on', async () => {
    const { agent, answerComplete } = fakeAgent('complete')
    const wrapper = mount(FeedbackCard, { props: { agent } })
    expect(wrapper.find('[data-testid="question"]').text()).toContain('complete your task')
    await wrapper.find('[data-testid="yes"]').trigger('click')
    await wrapper.find('[data-testid="no"]').trigger('click')
    await wrapper.find('[data-testid="skip"]').trigger('click')
    expect(answerComplete.mock.calls).toEqual([[true], [false], [null]])
  })

  it('asks why after a No and sends what was typed, or nothing when skipped', async () => {
    const { agent, answerWhy } = fakeAgent('why')
    const wrapper = mount(FeedbackCard, { props: { agent } })
    await wrapper.find('[data-testid="why"]').setValue('It typed in the wrong window')
    await wrapper.find('[data-testid="send-why"]').trigger('click')
    await wrapper.find('[data-testid="skip-why"]').trigger('click')
    expect(answerWhy.mock.calls).toEqual([['It typed in the wrong window'], ['']])
  })
})