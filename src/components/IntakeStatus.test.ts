import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import IntakeStatus from './IntakeStatus.vue'

it('makes classification visible and clears it when processing ends', async () => {
  const wrapper = mount(IntakeStatus, { props: { classifying: true, error: '' } })
  expect(wrapper.get('[role="status"]').text()).toContain('Understanding your request')
  expect(wrapper.findComponent({ name: 'RuneLoader' }).exists()).toBe(true)
  await wrapper.setProps({ classifying: false, error: 'Unable to load request status' })
  expect(wrapper.find('[role="status"]').exists()).toBe(false)
  expect(wrapper.get('[role="alert"]').text()).toBe('Unable to load request status')
  wrapper.unmount()
})