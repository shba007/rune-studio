import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { expect, it } from 'vite-plus/test'
import RuneChoice from './RuneChoice.vue'

it('places a scrollable model menu outside its clipped parent and selects an option', async () => {
  const host = document.createElement('div')
  document.body.appendChild(host)
  const wrapper = mount(RuneChoice, {
    attachTo: host,
    props: {
      modelValue: '',
      label: 'Model',
      menu: true,
      floating: true,
      options: [
        { value: '', label: 'Select a model', disabled: true },
        { value: 'model', label: 'A very long model identifier' },
      ],
    },
  })
  await wrapper.get('button[aria-label="Model"]').trigger('click')
  const menu = document.body.querySelector('[role="listbox"]') as HTMLElement
  expect(menu.parentElement).toBe(document.body)
  expect(menu.className).toContain('overflow-y-auto')
  ;(menu.querySelectorAll('[role="option"]')[1] as HTMLButtonElement).click()
  await nextTick()
  expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['model'])
  expect(document.body.querySelector('[role="listbox"]')).toBeNull()
  wrapper.unmount()
  host.remove()
})

it('skips disabled choices when using arrow keys', async () => {
  const wrapper = mount(RuneChoice, {
    props: {
      modelValue: '',
      label: 'Model',
      menu: true,
      options: [
        { value: '', label: 'Select', disabled: true },
        { value: 'model', label: 'Model' },
      ],
    },
  })
  await wrapper.get('button').trigger('keydown', { key: 'ArrowDown' })
  expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['model'])
  wrapper.unmount()
})

it('shows the model family icon for a selected LFM and its menu entry', async () => {
  const wrapper = mount(RuneChoice, {
    props: {
      modelValue: 'lfm3-omni-2b',
      label: 'Model',
      menu: true,
      modelIcons: true,
      options: [{ value: 'lfm3-omni-2b', label: 'LFM3' }],
    },
  })
  expect(
    wrapper.get('button[aria-label="Model"] span[aria-hidden="true"]').attributes('style'),
  ).toContain('/model/liquid.svg')
  await wrapper.get('button[aria-label="Model"]').trigger('click')
  expect(wrapper.get('[role="option"] span[aria-hidden="true"]').attributes('style')).toContain(
    '/model/liquid.svg',
  )
})