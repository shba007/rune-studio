import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, expect, it } from 'vite-plus/test'
import LifeGraph from './LifeGraph.vue'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'

const goal = (id: string, text: string, rest: Partial<IntakeEntry> = {}): IntakeEntry =>
  ({
    id,
    revision: 1,
    text,
    class: 'health',
    kind: 'goal',
    status: 'goal_ready',
    question: null,
    goal: null,
    approvedGoal: { outcome: text, horizon: '3 months' },
    error: null,
    taskId: null,
    ...rest,
  }) as IntakeEntry

const task = (id: string, title: string, parentGoalId: string): WorkspaceTask =>
  ({
    id,
    title,
    parentGoalId,
    executionMode: 'agent',
    totalSteps: 4,
    completedSteps: 2,
    occurrences: [],
  }) as unknown as WorkspaceTask

const props = () => ({
  entries: [
    goal('a', 'Run 5 km', { outcomeMetric: { baseline: 0, current: 5, target: 10 } as never }),
    goal('f', 'Plan family dinner', { class: 'general' }),
  ],
  tasks: [task('t1', 'Run-walk 20 minutes', 'a')],
})

beforeEach(() => localStorage.clear())

it('starts on the domains only, then zooms into goals and tasks, and back', async () => {
  const wrapper = mount(LifeGraph, { props: props() })
  expect(wrapper.get('[data-level]').attributes('data-level')).toBe('life')
  expect(wrapper.findAll('[data-node]').map((node) => node.attributes('data-node'))).toEqual([
    'physical',
    'emotional',
    'career',
    'financial',
    'environment',
  ])
  expect(wrapper.get('[data-node="physical"]').attributes('aria-label')).toContain('50%')
  await wrapper.get('[data-node="physical"]').trigger('click')
  await flushPromises()
  expect(wrapper.get('[data-level]').attributes('data-level')).toBe('domain')
  expect(wrapper.findAll('[data-node]').map((node) => node.attributes('data-node'))).toEqual(['a'])
  await wrapper.get('[data-node="a"]').trigger('click')
  await flushPromises()
  expect(wrapper.get('[data-level]').attributes('data-level')).toBe('goal')
  expect(wrapper.get('[data-node="t1"]').attributes('aria-label')).toContain('50%')
  await wrapper.get('[data-node="t1"]').trigger('click')
  expect(wrapper.emitted('openTask')).toEqual([['t1']])
  await wrapper
    .findAll('button')
    .find((button) => button.attributes('aria-label') === 'Zoom out')!
    .trigger('click')
  await flushPromises()
  expect(wrapper.get('[data-level]').attributes('data-level')).toBe('domain')
  wrapper.unmount()
})

it('shows Sanctuary domains without progress when enabled', async () => {
  localStorage.setItem('rune.showSanctuary', 'true')
  const wrapper = mount(LifeGraph, { props: props() })
  const node = wrapper.get('[data-node="family"]')
  expect(node.attributes('aria-label')).toBe('Family & Kinship, 1 goal')
  expect(node.find('circle').exists()).toBe(false)
  wrapper.unmount()
})