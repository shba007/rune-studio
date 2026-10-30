import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import MindsetSymbol from './MindsetSymbol.vue'
import { mindsetSymbols } from '@/config/mindsetSymbols'
it('renders symbols instead of letters for both current and legacy profiles', async () => {
  const wrapper = mount(MindsetSymbol, { props: { mark: 'R' } })
  expect(wrapper.text()).toBe('')
  expect(wrapper.get('path').attributes('d')).toBe(mindsetSymbols[0].path)
  await wrapper.setProps({ mark: 'fitness' })
  expect(wrapper.get('path').attributes('d')).toBe(mindsetSymbols[1].path)
  wrapper.unmount()
})