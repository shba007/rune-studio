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

it('has twelve domains with Cognitive in Sanctuary, following the approved taxonomy', () => {
  expect(DOMAINS).toHaveLength(12)
  expect(DOMAINS.filter((domain) => domain.zone === 'engine')).toHaveLength(5)
  expect(DOMAINS.find((domain) => domain.id === 'cognitive')!.zone).toBe('sanctuary')
})

it('follows the framework PDF: numbered domains with their sub-domains', () => {
  expect(DOMAINS.map((domain) => domain.number).sort((a, b) => a - b)).toEqual([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
  ])
  const counts = Object.fromEntries(DOMAINS.map((d) => [d.id, d.subDomains.length]))
  expect(counts).toMatchObject({ physical: 6, environment: 6, family: 5, legacy: 5 })
  expect(DOMAINS.find((domain) => domain.id === 'physical')!.subDomains[0]).toMatchObject({
    number: '1.1',
    name: 'Sleep & Circadian Architecture',
  })
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

it('uses stored domains before conflicting local overrides or inferred classifications', () => {
  expect(
    domainOf(goal('a', 'Run 5 km', { domain: 'career', class: 'health' }), { a: 'financial' }).id,
  ).toBe('career')
  expect(domainOf(goal('b', 'Run 5 km', { domain: null }), { b: 'career' }).id).toBe('career')
  expect(domainOf(goal('a', 'Run 5 km', { domain: 'unsupported' }), { a: 'financial' }).id).toBe(
    'unassigned',
  )
})

it('reveals Cognitive goals and numeric progress only when Sanctuary is enabled', () => {
  const entries = [
    goal('c', 'Learn something', {
      domain: 'cognitive',
      outcomeMetric: {
        baseline: 0,
        current: 5,
        target: 10,
        unit: 'chapters',
        note: '',
        updatedAt: '',
      },
    }),
  ]
  const hidden = mount(GoalsTable, { props: { entries, tasks: [] } })
  expect(hidden.find('[data-goal="c"]').exists()).toBe(false)
  hidden.unmount()
  localStorage.setItem('rune.showSanctuary', 'true')
  const shown = mount(GoalsTable, { props: { entries, tasks: [] } })
  expect(shown.get('[data-domain="cognitive"]').text()).toContain('50%')
  shown.unmount()
})
it('moves a goal to the domain chosen in overrides', async () => {
  localStorage.setItem('rune.goalDomains', JSON.stringify({ b: 'career' }))
  const wrapper = mount(GoalsTable, {
    props: { entries: [goal('b', 'Plan family dinner')], tasks: [] },
  })
  expect(wrapper.get('[data-domain="career"]').find('[data-goal="b"]').exists()).toBe(true)
  wrapper.unmount()
})