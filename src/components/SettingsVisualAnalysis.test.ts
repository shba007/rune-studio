import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { expect, it } from 'vite-plus/test'
import type { Settings } from '@/types/protocol'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'

it('keeps highlight settings in Studio and directs model configuration to Infer', async () => {
  const draft = reactive({}) as Settings
  const wrapper = mount(SettingsVisualAnalysis, { props: { draft } })
  expect(wrapper.text()).toContain('Configure Visual Analysis roles in the Infer catalog')
  expect(wrapper.find('button[aria-label="Detection model"]').exists()).toBe(false)
  await wrapper.get('button[aria-label="Depth"]').trigger('click')
  expect(draft.visualAnalysis?.depth).toBe(false)
  expect(draft.visualAnalysisModels).toBeUndefined()
})