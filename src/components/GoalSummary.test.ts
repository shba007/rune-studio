import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import GoalSummary from './GoalSummary.vue'
import type { IntakeEntry } from '@/types/decisions'
const goal = {
  outcome: 'Build a consistent fitness routine',
  current: 'No regular routine',
  success: 'Three workouts each week',
  horizon: '3 months',
  resources: 'Home equipment',
  constraints: 'Weekdays only',
}
function makeEntry(overrides: Partial<IntakeEntry>): IntakeEntry {
  return {
    id: 'goal-1',
    revision: 1,
    text: '',
    kind: 'goal',
    class: 'health',
    status: 'goal_ready',
    goal,
    approvedGoal: goal,
    question: null,
    error: null,
    taskId: null,
    ...overrides,
  }
}
it('presents the actual goal definition without inventing progress and preserves the original request', () => {
  const entry = makeEntry({
    text: 'I want to be physically fit',
    category: 'fitness',
    status: 'goal_ready',
    goal,
    approvedGoal: goal,
    question: null,
  })
  const wrapper = mount(GoalSummary, { props: { entry } })
  expect(wrapper.get('h2').text()).toBe(goal.outcome)
  for (const value of Object.values(goal)) expect(wrapper.text()).toContain(value)
  expect(wrapper.text()).toContain('Approved goal')
  expect(wrapper.get('details').text()).toContain(entry.text)
  expect(wrapper.find('[role="progressbar"]').exists()).toBe(false)
  wrapper.unmount()
})
it('distinguishes proposed changes from the saved definition during review', () => {
  const entry = makeEntry({
    text: 'Improve fitness',
    status: 'clarification',
    goal: { ...goal, horizon: '6 months' },
    approvedGoal: goal,
    question: {
      id: 'q',
      revision: 1,
      kind: 'clarification',
      title: 'What should change?',
      explanation: '',
      options: [],
    },
  })
  const wrapper = mount(GoalSummary, { props: { entry } })
  expect(wrapper.text()).toContain('Review changes')
  expect(wrapper.get('details').text()).toContain('3 months')
  expect(wrapper.text()).toContain('6 months')
  expect(wrapper.text()).not.toContain('Approved goal')
  wrapper.unmount()
})