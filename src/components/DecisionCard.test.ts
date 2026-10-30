import { mount } from '@vue/test-utils'
import { beforeEach, expect, it } from 'vite-plus/test'
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

beforeEach(() => localStorage.clear())

const domainQuestion: DecisionQuestion = {
  ...question,
  kind: 'domain',
  title: 'Which domain fits this goal?',
  options: [
    { id: 'career', label: 'Career & Professional Impact', explanation: 'May concern work.' },
    {
      id: 'cognitive',
      label: 'Cognitive & Intellectual Capital',
      explanation: 'May concern learning.',
    },
    { id: 'purpose', label: 'Character, Identity & Purpose', explanation: 'May concern meaning.' },
  ],
}

it('offers ranked suggestions and freeform without auto-confirming or revealing Sanctuary', async () => {
  const wrapper = mount(DecisionCard, { props: { question: domainQuestion } })
  expect(wrapper.findAll('input[type="radio"]')).toHaveLength(3)
  expect(wrapper.text()).toContain('Recommended')
  expect(wrapper.text()).toContain('Sanctuary · hidden in Workspace')
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  expect(wrapper.emitted('answer')).toBeUndefined()
  await wrapper.get('input[value="cognitive"]').setValue(true)
  expect(wrapper.emitted('answer')).toBeUndefined()
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: 'cognitive', text: '' },
  ])
  expect(localStorage.getItem('rune.showSanctuary')).toBe('false')
  await wrapper.get('textarea').setValue('I meant a different kind of personal direction')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    {
      id: question.id,
      revision: 1,
      optionId: null,
      text: 'I meant a different kind of personal direction',
    },
  ])
  wrapper.unmount()
})

it('offers all twelve domains through versioned freeform and clears stale picker state', async () => {
  const wrapper = mount(DecisionCard, { props: { question: domainQuestion } })
  await wrapper.get('button[aria-label="Another goal domain"]').trigger('click')
  expect(wrapper.findAll('[role="option"]')).toHaveLength(13)
  await wrapper
    .findAll('[role="option"]')
    .find((node) => node.text().startsWith('Family & Kinship'))!
    .trigger('click')
  expect(wrapper.emitted('answer')).toBeUndefined()
  expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe('family')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: null, text: 'family' },
  ])
  await wrapper.setProps({ question: { ...domainQuestion, revision: 2 }, busy: true })
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  expect(
    wrapper.get('button[aria-label="Another goal domain"]').attributes('disabled'),
  ).toBeDefined()
  expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe('')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')).toHaveLength(1)
  wrapper.unmount()
})

const subdomainQuestion: DecisionQuestion = {
  ...question,
  kind: 'subdomains',
  domain: 'environment',
  title: 'Which parts matter?',
  options: [
    {
      id: 'workspace-ergonomics',
      label: 'Workspace Friction',
      explanation: 'Your workspace may matter.',
    },
    {
      id: 'digital-hygiene',
      label: 'Digital Hygiene',
      explanation: 'Your stated distractions may matter.',
    },
    { id: 'geographic-fit', label: 'Sensory Fit', explanation: 'Your sensory context may matter.' },
  ],
}

it('lets the user select several sub-domains without exposing transport JSON in the form', async () => {
  const wrapper = mount(DecisionCard, { props: { question: subdomainQuestion } })
  expect(wrapper.findAll('[role="switch"]')).toHaveLength(6)
  expect(wrapper.findAll('input[type="radio"]')).toHaveLength(3)
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[role="switch"][aria-label="Digital Hygiene"]').trigger('click')
  await wrapper.get('[role="switch"][aria-label="Sensory Fit"]').trigger('click')
  expect(wrapper.emitted('answer')).toBeUndefined()
  expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe('')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: null, text: '["digital-hygiene","geographic-fit"]' },
  ])
  await wrapper.get('textarea').setValue('Noise seems to be the issue, but I am unsure')
  expect(
    wrapper.get('[role="switch"][aria-label="Digital Hygiene"]').attributes('aria-checked'),
  ).toBe('false')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    {
      id: question.id,
      revision: 1,
      optionId: null,
      text: 'Noise seems to be the issue, but I am unsure',
    },
  ])
  wrapper.unmount()
})

it('restores a multi-select draft and clears it on a new revision', async () => {
  const wrapper = mount(DecisionCard, {
    props: {
      question: {
        ...subdomainQuestion,
        draft: { optionId: null, text: '["geographic-fit"]' },
      },
    },
  })
  expect(wrapper.get('[role="switch"][aria-label="Sensory Fit"]').attributes('aria-checked')).toBe(
    'true',
  )
  expect((wrapper.get('textarea').element as HTMLTextAreaElement).value).toBe('')
  await wrapper.setProps({ question: { ...subdomainQuestion, revision: 2 } })
  expect(wrapper.get('button[type="submit"]').attributes('disabled')).toBeDefined()
  wrapper.unmount()
})

it('allows explicit skips without an answer and disables them while busy', async () => {
  const wrapper = mount(DecisionCard, {
    props: { question: { ...subdomainQuestion, kind: 'subdomain_detail' } },
  })
  await wrapper.get('textarea').setValue('An unfinished sensitive draft')
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Skip this part')!
    .trigger('click')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: 'skip', text: '' },
  ])
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Skip remaining questions')!
    .trigger('click')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: 'skip_all', text: '' },
  ])
  await wrapper.setProps({ busy: true })
  expect(
    wrapper
      .findAll('button')
      .find((b) => b.text() === 'Skip this part')!
      .attributes('disabled'),
  ).toBeDefined()
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Skip this part')!
    .trigger('click')
  expect(wrapper.emitted('answer')).toHaveLength(2)
  wrapper.unmount()
})

it('supports Auto discovery answers and skips without asking the user to type a goal', async () => {
  const discovery = {
    ...question,
    kind: 'discovery',
    title: 'How rested do you usually feel?',
    domain: 'physical',
  }
  const wrapper = mount(DecisionCard, { props: { question: discovery } })
  expect(wrapper.text()).not.toContain('What would you like to achieve')
  await wrapper.get('textarea').setValue('My current sleep routine varies')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: null, text: 'My current sleep routine varies' },
  ])
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Skip remaining questions')!
    .trigger('click')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: 'skip_all', text: '' },
  ])
  await wrapper.setProps({ question: { ...discovery, kind: 'discovery_required', revision: 2 } })
  expect(
    wrapper.findAll('button').some((button) => button.text() === 'Skip remaining questions'),
  ).toBe(false)
  expect(wrapper.find('textarea').exists()).toBe(true)
  wrapper.unmount()
})

it('allows Auto draft edits as changes with no direct goal approval option', async () => {
  const wrapper = mount(DecisionCard, {
    props: {
      question: {
        ...question,
        kind: 'discovery_review',
        title: "Review Rune's proposed goal",
        options: [
          {
            id: 'continue',
            label: 'Use draft and clarify',
            explanation: 'Clarify before goal approval.',
          },
          { id: 'refine', label: 'Edit the draft', explanation: 'Write requested changes.' },
          { id: 'more', label: 'Answer more questions', explanation: 'Gather more context.' },
        ],
      },
    },
  })
  expect(wrapper.text()).toContain('Your changes')
  expect(wrapper.find('input[value="approve"]').exists()).toBe(false)
  await wrapper.get('textarea').setValue('Keep the draft smaller')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('answer')!.at(-1)).toEqual([
    { id: question.id, revision: 1, optionId: null, text: 'Keep the draft smaller' },
  ])
  wrapper.unmount()
})

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