import { mount } from '@vue/test-utils'
import { beforeEach, expect, it } from 'vite-plus/test'
import GoalsTable from './GoalsTable.vue'
import { DOMAINS, domainOf } from '@/config/domains'
import type { IntakeEntry } from '@/types/decisions'

const goal = (id: string, text: string, rest: Partial<IntakeEntry> = {}): IntakeEntry =>
  ({
    id,
    revision: 1,
    text,
    class: 'general',
    kind: 'goal',
    status: 'goal_ready',
    question: null,
    goal: null,
    approvedGoal: { outcome: text, horizon: '3 months' },
    error: null,
    taskId: null,
    ...rest,
  }) as IntakeEntry

beforeEach(() => localStorage.clear())

it('has twelve domains, six per zone, in a fixed set', () => {
  expect(DOMAINS).toHaveLength(12)
  expect(DOMAINS.filter((domain) => domain.zone === 'engine')).toHaveLength(6)
})

it('maps goals by class and keywords, and honours an override', () => {
  expect(domainOf(goal('a', 'Run 5 km', { class: 'health' })).id).toBe('physical')
  expect(domainOf(goal('b', 'Save for a trip', { class: 'finance' })).id).toBe('financial')
  expect(domainOf(goal('c', 'Call my family more')).id).toBe('family')
  expect(domainOf(goal('d', 'Something vague')).id).toBe('unassigned')
  expect(domainOf(goal('d', 'Something vague'), { d: 'cognitive' }).id).toBe('cognitive')
})

it('shows every Engine domain, hides Sanctuary goals with a note, and filters', async () => {
  const entries = [
    goal('a', 'Run 5 km', { class: 'health' }),
    goal('b', 'Plan family dinner'),
    goal('c', 'Save money', { class: 'finance', question: { id: 'q' } as never }),
  ]
  const wrapper = mount(GoalsTable, { props: { entries, tasks: [] } })
  expect(wrapper.findAll('[data-domain]').map((node) => node.attributes('data-domain'))).toEqual([
    'physical',
    'emotional',
    'cognitive',
    'career',
    'financial',
    'environment',
  ])
  expect(wrapper.find('[data-goal="b"]').exists()).toBe(false)
  expect(wrapper.text()).toContain('1 goal in Sanctuary domains is hidden')
  await wrapper.get('[data-goal="a"]').trigger('click')
  expect(wrapper.emitted('open')).toEqual([['a']])
  await wrapper
    .findAll('[role="radio"]')
    .find((button) => button.text() === 'Needs answer')!
    .trigger('click')
  expect(wrapper.find('[data-goal="a"]').exists()).toBe(false)
  expect(wrapper.find('[data-goal="c"]').exists()).toBe(true)
  wrapper.unmount()
})