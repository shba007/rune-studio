import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import CaptureTargetPicker from './CaptureTargetPicker.vue'

it('filters targets without selecting one, and emits only an explicit choice', async () => {
  const wrapper = mount(CaptureTargetPicker, {
    props: {
      modelValue: '',
      options: [
        { target: { kind: 'screen', id: 1 }, label: 'Screen 1' },
        { target: { kind: 'window', id: 2 }, label: 'Project notes' },
      ],
    },
  })
  await wrapper.get('[aria-label="Choose capture target"]').trigger('click')
  await wrapper.get('input').setValue('notes')
  expect(wrapper.text()).not.toContain('Screen 1')
  expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  await wrapper
    .findAll('button')
    .find((button) => button.text() === 'Project notes')!
    .trigger('click')
  expect(wrapper.emitted('update:modelValue')).toEqual([
    [JSON.stringify({ kind: 'window', id: 2 })],
  ])
  expect(wrapper.get('[aria-label="Choose capture target"]').attributes('aria-expanded')).toBe(
    'false',
  )
})