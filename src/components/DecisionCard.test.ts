import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import DecisionCard from './DecisionCard.vue'
import type { DecisionQuestion } from '@/types/decisions'

const question: DecisionQuestion = {
  id: 'decision-1',
  revision: 1,
  kind: 'classification',
  title: 'How should Rune treat this request?',
  explanation: 'Choose its intended scope.',
  options: [
    { id: 'task', label: 'Task', explanation: 'A bounded piece of work.' },
    { id: 'goal', label: 'Goal', explanation: 'A lasting outcome.' },
    { id: 'step', label: 'Step', explanation: 'One action inside a task.' },
  ],
}

it('requires an explicit answer and includes the record revision when confirming', async () => {
  const wrapper = mount(DecisionCard, { props: { question } })
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  await wrapper.get('input[value="task"]').setValue(true)
  expect(wrapper.emitted('answer')).toBeUndefined()
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: 'decision-1', revision: 1, optionId: 'task', text: '' },
  ])
  expect(wrapper.find('textarea').exists()).toBe(false)
  await wrapper.setProps({ question: { ...question, kind: 'clarification', revision: 2 } })
  await wrapper.get('textarea').setValue('It combines several long-term outcomes')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    {
      id: 'decision-1',
      revision: 2,
      optionId: null,
      text: 'It combines several long-term outcomes',
    },
  ])
  wrapper.unmount()
})

it('restores a saved draft and clears a stale selection after the question changes', async () => {
  const wrapper = mount(DecisionCard, {
    props: { question: { ...question, draft: { optionId: 'goal', text: '' } } },
  })
  expect((wrapper.get('input[value="goal"]').element as HTMLInputElement).checked).toBe(true)
  await wrapper.setProps({ question: { ...question, revision: 2 } })
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  await wrapper.setProps({ busy: true })
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')).toBeUndefined()
  wrapper.unmount()
})