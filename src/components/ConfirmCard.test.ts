import { mount } from '@vue/test-utils'
import { ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import type { Agent } from '@/composables/useAgent'
import type { ConfirmRequestPayload } from '@/types/protocol'
import ConfirmCard from './ConfirmCard.vue'

describe('external session restart consent', () => {
  it('shows the retrying agent and task, and requires an explicit restart or keep-stopped answer', async () => {
    const question = ref<ConfirmRequestPayload | null>({
      id: 'restart-1',
      source: 'Codex',
      text: 'restart the stopped external-agent session',
      reason: 'Continue drafting the document',
      risk: 'medium',
      reasons: ['You pressed Stop.'],
      timeoutSecs: 120,
      kind: 'restart',
    })
    const answer = vi.fn<(approved: boolean) => void>()
    const agent = { question, answer } as unknown as Agent
    const wrapper = mount(ConfirmCard, { props: { agent } })
    expect(answer).not.toHaveBeenCalled()
    expect(wrapper.get('[role=alertdialog]').text()).toContain('Codex')
    expect(wrapper.text()).toContain('Continue drafting the document')
    const buttons = wrapper.findAll('button')
    expect(buttons.map((button) => button.text())).toEqual(['Keep stopped', 'Restart session'])
    await buttons[0]!.trigger('click')
    expect(answer).toHaveBeenLastCalledWith(false)
    await buttons[1]!.trigger('click')
    expect(answer).toHaveBeenLastCalledWith(true)
    question.value = null
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[role=alertdialog]').exists()).toBe(false)
    wrapper.unmount()
  })
})