import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import { taskTimeline } from '@/config/taskTimeline'
import { historyFixture } from './taskTimelineTestKit'
import FlightTaskTimeline from './FlightTaskTimeline.vue'

it('renders one row per task with shared playheads and seeks to that task step', async () => {
  const wrapper = mount(FlightTaskTimeline, {
    props: { timeline: taskTimeline(historyFixture)!, timeMs: 15000 },
  })
  expect(wrapper.findAll('[data-task-row]')).toHaveLength(3)
  expect(wrapper.findAll('[data-step-duration]').map((node) => node.text())).toEqual([
    '0:08',
    '0:11',
    '0:15',
  ])
  expect(wrapper.findAll('.timeline-dots')).toHaveLength(3)
  expect(
    wrapper
      .findAll('[data-task-playhead]')
      .every((node) => node.attributes('style')?.includes('50%')),
  ).toBe(true)
  await wrapper.get('[aria-label="Notepad, step 1: Type, cancelled"]').trigger('click')
  expect(wrapper.emitted('seek')!.at(-1)).toEqual([10000])
  await wrapper.get('[aria-label="Seek Translate"]').trigger('keydown', { key: 'ArrowRight' })
  expect(wrapper.emitted('seek')!.at(-1)).toEqual([16000])
  expect(wrapper.text()).toContain('failed')
  expect(wrapper.text()).toContain('cancelled')
  wrapper.unmount()
})