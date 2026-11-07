import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import ConnectionEditor from './ConnectionEditor.vue'

it('requires an API key for a new Infer connection', () => {
  const wrapper = mount(ConnectionEditor, { props: { connection: null, busy: false } })
  expect(wrapper.get('input[type="password"]').attributes('required')).toBeDefined()
  expect(wrapper.text()).toContain('Required. Stored in the system keyring.')
})