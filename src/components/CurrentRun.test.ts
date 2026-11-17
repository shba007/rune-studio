import { mount } from '@vue/test-utils'
import { ref, shallowRef } from 'vue'
import { expect, it, vi } from 'vite-plus/test'
import type { LiveStepsModel, LiveRunSnapshot } from '@/composables/useLiveSteps'
import type { LiveStep } from '@/config/liveSteps'
import CurrentRun from './CurrentRun.vue'
import type { Agent } from '@/composables/useAgent'
function fixture(): LiveStepsModel {
  const steps: LiveStep[] = [
    {
      taskId: 1,
      step: 1,
      tool: 'kit_tool',
      kind: 'tool',
      model: 'local',
      text: 'Time',
      detail: 'time__now',
      ok: true,
      startedAt: 1000,
      finishedAt: 2000,
      result: {
        text: 'raw data',
        structuredContent: { count: 4 },
        outputSchema: {
          type: 'object',
          properties: { count: { type: 'integer', title: 'Count' } },
        },
      },
    },
    {
      taskId: 1,
      step: 2,
      tool: 'click',
      kind: 'screen',
      model: 'local',
      text: 'Click save',
      detail: 'Save',
      ok: true,
      startedAt: 2000,
      finishedAt: 3000,
    },
  ]
  const snapshot: LiveRunSnapshot = {
    revision: 1,
    request: {
      id: 1,
      text: 'Check the time',
      phase: 'running',
      dependsOn: [],
      error: null,
      goals: [],
      startedAt: 1000,
    },
    tasks: [
      {
        id: 1,
        requestId: 1,
        goalId: 'g',
        kind: 'background',
        text: 'Task',
        phase: 'running',
        steps: [],
        dependsOn: [],
        result: null,
      },
    ],
    steps: [],
    droppedResults: 0,
    droppedSteps: 0,
  }
  return {
    question: shallowRef(null),
    choices: shallowRef([]),
    steps: shallowRef(steps),
    snapshot: shallowRef(snapshot),
    error: ref(''),
    clearTask: vi.fn<(id: number | null) => void>(),
    refresh: vi.fn<() => Promise<void>>(async () => {}),
  }
}
it('shows only a current request and shares row/lane selection with a full schema view and no Raw', async () => {
  const live = fixture()
  const wrapper = mount(CurrentRun, { props: { live } })
  expect(wrapper.get('[aria-label="Selected live step"]').attributes('data-selected-step')).toBe(
    '1:2',
  )
  expect(wrapper.find('[aria-label="Live tool result"]').exists()).toBe(false)
  expect(wrapper.find('[data-testid="lane-detail"]').exists()).toBe(false)
  await wrapper.get('[data-step-key="1:1"] button').trigger('click')
  expect(wrapper.get('.block-mark[aria-pressed="true"]').attributes('data-kind')).toBe('tool')
  expect(wrapper.get('[aria-label="Live tool result"] dt').text()).toBe('Count')
  expect(wrapper.get('[aria-label="Live tool result"] dd').text()).toBe('4')
  expect(wrapper.find('[aria-label="Live tool result"] details').exists()).toBe(false)
  expect(wrapper.find('.compact-result').exists()).toBe(false)
  await wrapper.findAll('.block-mark')[1]!.trigger('click')
  expect(wrapper.get('[data-step-key="1:2"]').classes()).toContain('bg-rune-raised')
  expect(wrapper.find('[aria-label="Live tool result"]').exists()).toBe(false)
  live.snapshot.value = null
  await wrapper.vm.$nextTick()
  expect(wrapper.find('[aria-label="Current run"]').exists()).toBe(false)
  wrapper.unmount()
})
it('follows new steps until picked, accepts Open selection, and renders errors', async () => {
  const live = fixture()
  const wrapper = mount(CurrentRun, { props: { live } })
  const next: LiveStep = {
    ...live.steps.value[0]!,
    step: 3,
    startedAt: 3000,
    ok: false,
    error: 'Tool refused',
    result: undefined,
  }
  live.steps.value = [...live.steps.value, next]
  await wrapper.vm.$nextTick()
  expect(wrapper.get('[role="alert"]').text()).toBe('Tool refused')
  await wrapper.get('[data-step-key="1:1"] button').trigger('click')
  live.steps.value = [...live.steps.value, { ...next, step: 4, startedAt: 4000 }]
  await wrapper.vm.$nextTick()
  expect(wrapper.get('[aria-label="Selected live step"]').attributes('data-selected-step')).toBe(
    '1:1',
  )
  await wrapper.setProps({ focus: { key: '1:3', serial: 1 } })
  expect(wrapper.get('[role="alert"]').text()).toBe('Tool refused')
  live.snapshot.value = {
    ...live.snapshot.value!,
    request: { ...live.snapshot.value!.request!, phase: 'done', finishedAt: 5000 },
  }
  await wrapper.vm.$nextTick()
  expect(wrapper.find('[aria-label="Current run"]').exists()).toBe(true)
  wrapper.unmount()
})

it('selects and marks a waiting step, shows its latest tool content, then records the choice', async () => {
  const live = fixture()
  const confirmation = {
    question: live.question,
    steps: live.steps,
    answer: vi.fn<(approved: boolean) => Promise<void>>(async () => {}),
    choose: vi.fn<(index: number) => Promise<void>>(async (index) => {
      const question = live.question.value!
      live.choices.value = [
        {
          id: question.id,
          taskId: question.taskId,
          step: question.step,
          label: question.options![index]!.label,
          at: 4000,
        },
      ]
      live.question.value = null
    }),
  } as unknown as Agent
  const wrapper = mount(CurrentRun, { props: { live, confirmation } })
  live.question.value = {
    id: 'q',
    source: 'Rune',
    text: 'Use this result?',
    reason: '',
    reasons: [],
    risk: 'low',
    kind: 'question',
    timeoutSecs: 120,
    taskId: 1,
    step: 2,
    options: [
      { label: 'Keep', outcome: 'Keep result' },
      { label: 'Review', outcome: 'Review result' },
    ],
    suggested: 0,
  }
  await wrapper.vm.$nextTick()
  expect(wrapper.get('[aria-label="Selected live step"]').attributes('data-selected-step')).toBe(
    '1:2',
  )
  expect(wrapper.get('.block-waiting').attributes('data-kind')).toBe('screen')
  expect(wrapper.get('[data-step-key="1:2"]').classes()).toContain('ring-attention')
  expect(wrapper.get('[role="alertdialog"] dt').text()).toBe('Count')
  expect(wrapper.get('[role="alertdialog"] dd').text()).toBe('4')
  await wrapper.get('[role="alertdialog"] button[data-suggested="true"]').trigger('click')
  expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
  expect(wrapper.get('[data-testid="chosen-answer"]').text()).toBe('You chose Keep')
  expect(wrapper.find('.block-waiting').exists()).toBe(false)
  wrapper.unmount()
})