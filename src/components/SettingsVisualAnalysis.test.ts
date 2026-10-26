import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import type { Settings } from '@/types/protocol'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'

describe('Visual Analysis display settings', () => {
  it('defaults all groups on and changes only the selected checkbox', async () => {
    const draft = reactive({}) as Settings
    const wrapper = mount(SettingsVisualAnalysis, { props: { draft } })
    expect(
      wrapper.findAll('input').every((input) => (input.element as HTMLInputElement).checked),
    ).toBe(true)
    await wrapper.get('input[aria-label="Depth"]').setValue(false)
    expect(draft.visualAnalysis).toEqual({
      entities: true,
      regions: true,
      structure: true,
      depth: false,
    })
    await wrapper.get('input[aria-label="Depth"]').setValue(true)
    expect(draft.visualAnalysis?.depth).toBe(true)
    wrapper.unmount()
  })
})