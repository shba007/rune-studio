import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import type { LiveStep } from '@/config/liveSteps'
import LiveStepRows from './LiveStepRows.vue'
import AppIcon from './AppIcon.vue'

it('keeps rows compact, distinguishes kinds and shows models only at transitions', async () => {
  const steps: LiveStep[] = ['alpha', 'alpha', 'beta', 'beta'].map((model, index) => ({
    taskId: 1,
    step: index + 1,
    tool: 'test',
    text: `Action ${index}`,
    detail: 'Details',
    model,
    kind: index === 0 ? 'tool' : 'screen',
    ok: index === 0 ? true : index === 1 ? false : null,
  }))
  const wrapper = mount(LiveStepRows, { props: { steps } })
  expect(wrapper.findAll('li')).toHaveLength(4)
  expect(wrapper.findAll('[title="alpha"]')).toHaveLength(1)
  expect(wrapper.findAll('[title="beta"]')).toHaveLength(1)
  expect(wrapper.get('[aria-label="Tool"]').classes()).toContain('text-tool')
  expect(wrapper.findAll('[aria-label="Screen"]')[0]!.classes()).toContain('text-voice')
  expect(wrapper.findAllComponents(AppIcon).map((icon) => icon.props('name'))).toEqual([
    'plug',
    'mouse',
    'mouse',
    'mouse',
  ])
  expect(wrapper.text()).toContain('Done')
  expect(wrapper.text()).toContain('Failed')
  expect(wrapper.text()).toContain('Running')
  await wrapper.setProps({ steps: [{ ...steps[0]!, startedAt: 0, finishedAt: 2500 }] })
  expect(wrapper.text()).toContain('2.5 s')
  wrapper.unmount()
})