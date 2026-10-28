import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import ProgressRing from './ProgressRing.vue'

it('shows steady measured task progress instead of a full pulsing goal ring', async () => {
  const wrapper = mount(ProgressRing, {
    props: { goals: 1, current: 0, finished: false, running: true, failed: false, progress: 0.5 },
  })
  expect(wrapper.find('.planning-ring').exists()).toBe(false)
  expect(wrapper.find('.current').exists()).toBe(false)
  expect(wrapper.get('.task-progress').attributes('stroke-dasharray')).toBe('50 100')
  await wrapper.setProps({ progress: 0 })
  expect(wrapper.find('.task-progress').exists()).toBe(false)
  await wrapper.setProps({ progress: 1, running: false })
  expect(wrapper.get('.task-progress').attributes('stroke-dasharray')).toBe('100 100')
})