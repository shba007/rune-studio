import { flushPromises, mount } from '@vue/test-utils'
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
    expect(buttons.map((button) => button.text())).toEqual(['Keep stopped', 'Restart task'])
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

it('shows option outcomes with the suggestion focused and sends the chosen index', async () => {
  const question = ref<ConfirmRequestPayload | null>({
    id: 'q',
    source: 'Rune',
    text: 'Choose how to use this',
    reason: '',
    risk: 'low',
    reasons: [],
    timeoutSecs: 120,
    kind: 'question',
    taskId: 1,
    step: 2,
    options: [
      { label: 'Keep', outcome: 'Keep this content' },
      { label: 'Review', outcome: 'Review it first' },
    ],
    suggested: 1,
  })
  const choose = vi.fn<(index: number) => Promise<void>>(async () => {})
  const answer = vi.fn<(approved: boolean) => Promise<void>>(async () => {})
  const wrapper = mount(ConfirmCard, {
    attachTo: document.body,
    props: { agent: { question, choose, answer } },
  })
  await flushPromises()
  const suggested = wrapper.get('button[data-suggested="true"]')
  expect(suggested.text()).toContain('Review')
  expect(suggested.text()).toContain('Suggested')
  expect(suggested.text()).toContain('Review it first')
  expect(suggested.classes()).toContain('bg-rune-ink')
  expect(document.activeElement).toBe(suggested.element)
  await wrapper.get('button[data-suggested="false"]').trigger('click')
  expect(choose).toHaveBeenCalledExactlyOnceWith(0)
  expect(answer).not.toHaveBeenCalled()
  wrapper.unmount()
})

it('keeps yes/no for legacy questions and never offers choices for approvals', async () => {
  const question = ref<ConfirmRequestPayload | null>({
    id: 'q',
    source: 'Rune',
    text: 'Continue?',
    reason: '',
    risk: 'low',
    reasons: [],
    timeoutSecs: 120,
    kind: 'question',
  })
  const answer = vi.fn<(approved: boolean) => Promise<void>>(async () => {})
  const wrapper = mount(ConfirmCard, { props: { agent: { question, answer } } })
  expect(wrapper.findAll('button').map((button) => button.text())).toEqual(['No', 'Yes'])
  await wrapper.findAll('button')[1]!.trigger('click')
  expect(answer).toHaveBeenCalledWith(true)
  question.value = {
    ...question.value!,
    kind: 'approval',
    options: [
      { label: 'Keep', outcome: 'Keep it' },
      { label: 'Review', outcome: 'Review it' },
    ],
    suggested: 0,
  }
  await wrapper.vm.$nextTick()
  expect(wrapper.findAll('button').map((button) => button.text())).toEqual(['No', 'Yes, allow'])
  expect(wrapper.find('[data-suggested]').exists()).toBe(false)
  wrapper.unmount()
})