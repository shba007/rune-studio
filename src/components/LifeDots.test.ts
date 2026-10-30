import { mount } from '@vue/test-utils'
import { beforeEach, expect, it } from 'vite-plus/test'
import LifeDots from './LifeDots.vue'
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

const metric = (current: number) => ({ baseline: 0, current, target: 10, unit: 'x' }) as never

beforeEach(() => localStorage.clear())

it('shows nothing until a goal exists', () => {
  const wrapper = mount(LifeDots, { props: { entries: [] } })
  expect(wrapper.find('[data-testid="life-dots"]').exists()).toBe(false)
})

it('shows only the five Engine domains, brighter with progress, and opens Goals', async () => {
  const entries = [
    goal('a', 'Run 5 km', { class: 'health', outcomeMetric: metric(8) }),
    goal('b', 'Save money', { class: 'finance', outcomeMetric: metric(2) }),
    goal('c', 'Plan family dinner'),
  ]
  const wrapper = mount(LifeDots, { props: { entries } })
  const dots = wrapper.findAll('[data-dot]')
  expect(dots.map((dot) => dot.attributes('data-dot'))).toEqual([
    'physical',
    'emotional',
    'career',
    'financial',
    'environment',
  ])
  const level = (id: string) =>
    Number((wrapper.get(`[data-dot="${id}"]`).element as HTMLElement).style.opacity)
  expect(level('physical')).toBeGreaterThan(level('financial'))
  expect(level('financial')).toBeGreaterThan(level('emotional'))
  expect(wrapper.get('[data-dot="financial"]').classes()).toContain('motion-safe:animate-pulse')
  expect(wrapper.get('[data-testid="life-dots"]').attributes('aria-label')).toContain(
    'Physical Vitality: 80%',
  )
  await wrapper.get('[data-testid="life-dots"]').trigger('click')
  expect(wrapper.emitted('open')).toHaveLength(1)
  wrapper.unmount()
})

it('shows Sanctuary progress dots only when the Settings switch is enabled', () => {
  const entries = [goal('c', 'Learn Rust', { domain: 'cognitive', outcomeMetric: metric(5) })]
  const hidden = mount(LifeDots, { props: { entries } })
  expect(hidden.find('[data-testid="life-dots"]').exists()).toBe(false)
  hidden.unmount()
  localStorage.setItem('rune.showSanctuary', 'true')
  const shown = mount(LifeDots, { props: { entries } })
  expect(shown.findAll('[data-dot]')).toHaveLength(12)
  expect(shown.get('[data-testid="life-dots"]').attributes('aria-label')).toContain(
    'Cognitive & Intellectual Capital: 50%',
  )
  shown.unmount()
})