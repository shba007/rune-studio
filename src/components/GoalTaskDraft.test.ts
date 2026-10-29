import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import GoalTaskDraft from './GoalTaskDraft.vue'
import type { GoalTaskDraft as Draft, GoalPathway, IntakeEntry } from '@/types/decisions'
function entry(): IntakeEntry {
  const route: GoalPathway = {
    id: 'route-1',
    title: 'Gradual plan',
    summary: 'Build consistency',
    rationale: 'Fits availability',
    pros: [],
    cons: [],
    duration: 'One month',
    resources: [],
    feasibility: 'conditional',
    missingInformation: [],
  }
  const goal = { outcome: 'Build consistency' }
  const draft: Draft = {
    id: 'draft',
    goal,
    pathway: route,
    startLocal: '2028-02-27T12:00',
    endLocal: '2028-03-05T12:00',
    zone: 'UTC',
    assessment: 'Start with supported work',
    tasks: [
      {
        id: 'task-1',
        title: 'Short walk',
        instructions: 'Walk for the agreed duration',
        rationale: 'Uses stated availability',
        executionMode: 'manual',
        schedule: { local: '2028-02-29T07:00', zone: 'UTC', repeat: 'weekly', anchorDay: 29 },
        createdTaskId: null,
        dismissed: false,
      },
    ],
  }
  return {
    id: 'goal',
    revision: 9,
    text: 'Build consistency',
    class: 'health',
    kind: 'goal',
    status: 'goal_ready',
    question: null,
    goal,
    approvedGoal: goal,
    pathways: {
      assessment: 'Supported',
      similarOutcome: '',
      options: [route],
      selected: route.id,
      approved: route,
    },
    taskProposals: draft,
    error: null,
    taskId: null,
  }
}
it('generates only after a click and approves edited instructions and schedules separately', async () => {
  const wrapper = mount(GoalTaskDraft, { props: { entry: entry(), busy: false } })
  expect(wrapper.emitted('review')).toBeUndefined()
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Draft new proposals')!
    .trigger('click')
  expect(wrapper.emitted('generate')).toEqual([[]])
  expect(wrapper.emitted('review')).toBeUndefined()
  await wrapper.get('textarea').setValue('Walk for ten minutes, if comfortable')
  await wrapper.get('input[type="datetime-local"]').setValue('2028-03-01T08:30')
  await wrapper
    .findAll('[role="radio"]')
    .find((b) => b.text() === 'Daily')!
    .trigger('click')
  await wrapper.get('form').trigger('submit')
  expect(wrapper.emitted('review')).toEqual([
    [
      'task-1',
      {
        title: 'Short walk',
        instructions: 'Walk for ten minutes, if comfortable',
        executionMode: 'manual',
        schedule: { local: '2028-03-01T08:30', zone: 'UTC', repeat: 'daily', anchorDay: null },
      },
    ],
  ])
  expect(wrapper.text()).toContain('continues beyond the first week')
  wrapper.unmount()
})
it('allows removing the schedule and dismissing a proposal without execution', async () => {
  const wrapper = mount(GoalTaskDraft, { props: { entry: entry(), busy: false } })
  await wrapper.get('[role="switch"]').trigger('click')
  await wrapper.get('form').trigger('submit')
  expect((wrapper.emitted('review')![0]![1] as { schedule: unknown }).schedule).toBeNull()
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Dismiss proposal')!
    .trigger('click')
  expect(wrapper.emitted('review')!.at(-1)).toEqual(['task-1', null])
  wrapper.unmount()
})
it('blocks stale drafts and links created work to its existing task', async () => {
  const value = entry()
  const wrapper = mount(GoalTaskDraft, { props: { entry: value, busy: false } })
  await wrapper.setProps({ entry: { ...value, approvedGoal: { outcome: 'Changed' } } })
  expect(wrapper.get('[role="alert"]').text()).toContain('earlier goal or pathway')
  expect(wrapper.get('fieldset').attributes('disabled')).toBeDefined()
  value.taskProposals!.tasks[0]!.createdTaskId = 'worktask-existing'
  await wrapper.setProps({ entry: { ...value } })
  await wrapper
    .findAll('button')
    .find((b) => b.text() === 'Task created →')!
    .trigger('click')
  expect(wrapper.emitted('open')).toEqual([['worktask-existing']])
  expect(wrapper.find('form').exists()).toBe(false)
  wrapper.unmount()
})