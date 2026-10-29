import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { expect, it } from 'vite-plus/test'
import SettingsGoalContext from './SettingsGoalContext.vue'
import type { Settings } from '@/types/protocol'
it('defaults to Automatic and requires explicit opt-in for each saved domain', async () => {
  const draft = reactive({}) as Settings
  const wrapper = mount(SettingsGoalContext, { props: { draft } })
  expect(draft.inputClassification).toBe('automatic')
  await wrapper
    .findAll('[role="radio"]')
    .find((button) => button.text() === 'Always confirm')!
    .trigger('click')
  expect(draft.inputClassification).toBe('always_confirm')
  expect(wrapper.find('[aria-label="Allow health context"]').exists()).toBe(false)
  await wrapper.get('[aria-label="Save goal conversations"]').trigger('click')
  expect(draft.lifeEnabled).toBe(true)
  expect(draft.lifeConsents!.health).toBe(false)
  await wrapper.get('[aria-label="Allow health context"]').trigger('click')
  expect(draft.lifeConsents!.health).toBe(true)
  expect(draft.lifeConsents!.finance).toBe(false)
  wrapper.unmount()
})