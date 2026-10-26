import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import GoalPathways from './GoalPathways.vue'
import type { IntakeEntry, GoalPathway } from '@/types/decisions'
it('shows route trade-offs, keeps selection explicit, and blocks unassessed or infeasible choices', async () => {
  const route: GoalPathway = {
    id: 'route-1',
    title: 'Gradual route',
    summary: 'Small increments',
    rationale: 'Fits supplied time',
    pros: ['Flexible'],
    cons: ['Slower'],
    duration: 'One month, uncertain',
    resources: ['Time'],
    feasibility: 'conditional',
    missingInformation: ['Available days'],
  }
  const entry = {
    status: 'goal_ready',
    pathways: {
      assessment: 'Estimates only',
      similarOutcome: '',
      selected: null,
      approved: null,
      options: [
        route,
        { ...route, id: 'route-2', feasibility: 'infeasible' },
        { ...route, id: 'route-3', feasibility: 'unknown' },
      ],
    },
  } as IntakeEntry
  const wrapper = mount(GoalPathways, { props: { entry, busy: false } })
  expect(wrapper.text()).toContain('Available days')
  expect(wrapper.text()).toContain('does not create tasks')
  expect(wrapper.emitted('select')).toBeUndefined()
  const buttons = wrapper.findAll('button')
  expect(buttons[1]!.attributes('disabled')).toBeDefined()
  expect(buttons[2]!.attributes('disabled')).toBeDefined()
  await buttons[0]!.trigger('click')
  expect(wrapper.emitted('select')).toEqual([['route-1']])
  await wrapper.setProps({ busy: true })
  expect(wrapper.get('button').attributes('disabled')).toBeDefined()
  wrapper.unmount()
})