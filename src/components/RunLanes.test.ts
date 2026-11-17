import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import type { LiveStep } from '@/config/liveSteps'
import RunLanes from './RunLanes.vue'
import ToolResultView from './ToolResultView.vue'
import { vi } from 'vite-plus/test'

const step = (over: Partial<LiveStep>): LiveStep => ({
  taskId: 1,
  step: 1,
  kind: 'screen',
  model: 'local',
  tool: 'click',
  text: 'Click Save',
  detail: 'Save button',
  ok: true,
  startedAt: 0,
  finishedAt: 1000,
  ...over,
})

it('places each task on its own lane, colours by kind and explains the picked step', async () => {
  const steps = [
    step({ kind: 'tool', tool: 'kit_tool', text: 'Get the time', detail: 'time__now' }),
    step({ taskId: 2, startedAt: 1000, finishedAt: 3000 }),
    step({ taskId: 2, step: 2, text: 'Type text', startedAt: 3000, finishedAt: null, ok: null }),
    step({ taskId: 9, text: 'Other request' }),
  ]
  const wrapper = mount(RunLanes, {
    props: {
      lanes: [
        { id: 1, label: 'Task 1' },
        { id: 2, label: 'Task 2' },
      ],
      steps,
      now: 4000,
    },
  })
  const kindsOn = (lane: number) =>
    wrapper.findAll(`[data-lane="${lane}"] .block-mark`).map((b) => b.attributes('data-kind'))
  expect(kindsOn(1)).toEqual(['tool'])
  expect(kindsOn(2)).toEqual(['screen', 'screen'])
  expect(wrapper.text()).not.toContain('Other request')
  expect(wrapper.text()).toContain('Tool call')
  expect(wrapper.text()).toContain('Screen action')
  expect(wrapper.get('[data-testid="lane-span"]').text()).toBe('4.0 s')

  const second = wrapper.findAll('[data-lane="2"] .block-mark')[0]!
  expect(second.attributes('style')).toContain('left: 25%')
  expect(second.attributes('style')).toContain('width: 50%')

  const detail = wrapper.get('[data-testid="lane-detail"]')
  expect(detail.text()).toContain('Type text')
  expect(detail.text()).toContain('Running')

  await wrapper.get('[data-lane="1"] .block-mark').trigger('click')
  expect(detail.text()).toContain('Get the time')
  expect(detail.text()).toContain('time__now')
  expect(detail.text()).toContain('local')
  expect(detail.text()).toContain('1.0 s')
  wrapper.unmount()
})

it('stays hidden until a step has started', () => {
  const wrapper = mount(RunLanes, {
    props: { lanes: [{ id: 1, label: 'Task 1' }], steps: [], now: 0 },
  })
  expect(wrapper.find('[aria-label="Run timeline"]').exists()).toBe(false)
  wrapper.unmount()
})

it('opens the selected step through the supplied Workspace path', async () => {
  const openStep = vi.fn<(key: string) => void>()
  const wrapper = mount(RunLanes, {
    props: { lanes: [{ id: 1, label: 'Task' }], steps: [step({})], now: 2000, openStep },
  })
  await wrapper.get('[data-testid="lane-detail"] button').trigger('click')
  expect(openStep).toHaveBeenCalledExactlyOnceWith('1:1')
  wrapper.unmount()
})

it('opens a tool result with schema in the existing detail card, compact and without a raw view', async () => {
  const result = {
    text: 'Original tool output',
    structuredContent: { count: 4 },
    outputSchema: {
      type: 'object',
      properties: { count: { type: 'integer', title: 'Item count' } },
    },
  }
  const wrapper = mount(RunLanes, {
    props: {
      lanes: [{ id: 1, label: 'Task' }],
      steps: [step({ kind: 'tool', result }), step({ step: 2 })],
      now: 2000,
    },
  })
  expect(wrapper.find('[data-testid="lane-result"]').exists()).toBe(false)
  expect(wrapper.findAll('[data-testid="lane-result-mark"]')).toHaveLength(1)
  expect(wrapper.findAll('.block-mark')[0]!.attributes('aria-label')).toContain('Result available')
  await wrapper.findAll('.block-mark')[0]!.trigger('click')
  const area = wrapper.get('[data-testid="lane-detail"] [data-testid="lane-result"]')
  expect(area.get('dt').text()).toBe('Item count')
  expect(area.get('dd').text()).toBe('4')
  expect(wrapper.getComponent(ToolResultView).props('compact')).toBe(true)
  expect(area.get('.compact-result').classes()).toContain('max-h-64')
  expect(area.get('.compact-result').classes()).toContain('overflow-y-auto')
  expect(area.find('button').exists()).toBe(false)
  expect(area.find('details').exists()).toBe(false)
  expect(area.text()).not.toContain('Original tool output')
  await wrapper.findAll('.block-mark')[1]!.trigger('click')
  expect(wrapper.find('[data-testid="lane-result"]').exists()).toBe(false)
  wrapper.unmount()
})

it('shows legacy text without a schema and the omission hint', () => {
  const wrapper = mount(RunLanes, {
    props: {
      lanes: [{ id: 1, label: 'Task' }],
      steps: [
        step({
          kind: 'tool',
          result: { text: 'Plain returned text', structuredContentOmitted: true },
        }),
      ],
      now: 2000,
    },
  })
  const area = wrapper.get('[data-testid="lane-result"]')
  expect(area.text()).toContain('Plain returned text')
  expect(area.text()).toContain('Structured result omitted for size')
  wrapper.unmount()
})

it('shows failed tool errors and never renders results for screen steps', async () => {
  const result = { text: 'Do not show this instead of the error' }
  const wrapper = mount(RunLanes, {
    props: {
      lanes: [{ id: 1, label: 'Task' }],
      steps: [
        step({ kind: 'tool', ok: false, error: 'Kit tool error: refused', result }),
        step({ step: 2, kind: 'screen', result }),
      ],
      now: 2000,
    },
  })
  expect(wrapper.find('[data-testid="lane-result"]').exists()).toBe(false)
  await wrapper.findAll('.block-mark')[0]!.trigger('click')
  expect(wrapper.get('[role="alert"]').text()).toBe('Kit tool error: refused')
  expect(wrapper.findComponent(ToolResultView).exists()).toBe(false)
  await wrapper.setProps({
    steps: [step({ kind: 'tool', ok: false, error: 'No result returned' })],
  })
  expect(wrapper.get('[role="alert"]').text()).toBe('No result returned')
  wrapper.unmount()
})
it('shows a Kit tool by its own name with the server in the meta line', () => {
  const wrapper = mount(RunLanes, {
    props: {
      lanes: [{ id: 1, label: 'Task' }],
      steps: [
        step({ kind: 'tool', text: 'kit__rune-time__get_current_time', model: 'local-model' }),
      ],
      now: 2000,
    },
  })
  const detail = wrapper.get('[data-testid="lane-detail"]')
  expect(detail.get('[title="kit__rune-time__get_current_time"]').text()).toBe('get_current_time')
  expect(detail.get('[data-testid="lane-meta"]').text()).toBe('Tool call · rune-time · local-model')
  wrapper.unmount()
})