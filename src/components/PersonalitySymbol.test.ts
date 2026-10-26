import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import PersonalitySymbol from './PersonalitySymbol.vue'
import { personalitySymbols } from '@/config/personalitySymbols'
it('renders symbols instead of letters for both current and legacy profiles', async () => {
  const wrapper = mount(PersonalitySymbol, { props: { mark: 'R' } })
  expect(wrapper.text()).toBe('')
  expect(wrapper.get('path').attributes('d')).toBe(personalitySymbols[0].path)
  await wrapper.setProps({ mark: 'fitness' })
  expect(wrapper.get('path').attributes('d')).toBe(personalitySymbols[1].path)
  wrapper.unmount()
})