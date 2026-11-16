import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import type { LiveStep } from '@/config/liveSteps'
import RunLanes from './RunLanes.vue'

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