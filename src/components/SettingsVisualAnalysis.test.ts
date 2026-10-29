import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import type { Settings } from '@/types/protocol'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'

describe('Visual Analysis display settings', () => {
  it('defaults all groups on and changes only the selected switch', async () => {
    const draft = reactive({}) as Settings
    const wrapper = mount(SettingsVisualAnalysis, { props: { draft } })
    expect(
      wrapper
        .findAll('button[role="switch"]')
        .every((item) => item.attributes('aria-checked') === 'true'),
    ).toBe(true)
    await wrapper.get('button[aria-label="Depth"]').trigger('click')
    expect(draft.visualAnalysis).toEqual({
      entities: true,
      regions: true,
      structure: true,
      depth: false,
    })
    await wrapper.get('button[aria-label="Depth"]').trigger('click')
    expect(draft.visualAnalysis?.depth).toBe(true)
    wrapper.unmount()
  })
})