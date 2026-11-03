import { mount } from '@vue/test-utils'
import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import type { ModelCapabilities, Settings } from '@/types/protocol'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'

describe('Visual Analysis display settings', () => {
  it('separates boxes, instance masks, oriented boxes and pose models', async () => {
    const draft = reactive({}) as Settings
    const models = [
      { id: 'boxes', tasks: ['image-to-detection'], rawCapabilities: [] },
      { id: 'pose', tasks: ['image-to-detection'], rawCapabilities: ['Pose Estimation'] },
      { id: 'mask', tasks: ['image-to-detection'], rawCapabilities: ['Instance Segmentation'] },
      {
        id: 'oriented',
        tasks: ['image-to-detection'],
        rawCapabilities: ['Oriented Bounding Boxes'],
      },
    ] as unknown as ModelCapabilities[]
    const wrapper = mount(SettingsVisualAnalysis, { props: { draft, models } })
    for (const [name, expected] of [
      ['Detection model', 'boxes'],
      ['Pose model', 'pose'],
      ['Instance segmentation model', 'mask'],
      ['Oriented detection model', 'oriented'],
    ]) {
      await wrapper.get(`button[aria-label="${name}"]`).trigger('click')
      const menu = wrapper.get(`[role="listbox"][aria-label="${name}"]`)
      expect(menu.findAll('[role="option"]').map((item) => item.text())).toEqual([
        'Disabled',
        expected,
      ])
      await wrapper.get(`button[aria-label="${name}"]`).trigger('click')
    }
    wrapper.unmount()
  })
  it('preserves an incompatible prior choice and asks for a compatible model', () => {
    const draft = reactive({
      visualAnalysisModels: {
        planner: '',
        entities: 'pose',
        regions: '',
        structure: '',
        depth: '',
      },
    }) as Settings
    const models = [
      { id: 'pose', tasks: ['image-to-detection'], rawCapabilities: ['Pose Estimation'] },
    ] as unknown as ModelCapabilities[]
    const wrapper = mount(SettingsVisualAnalysis, { props: { draft, models } })
    expect(wrapper.get('button[aria-label="Detection model"]').text()).toContain('incompatible')
    expect(draft.visualAnalysisModels?.entities).toBe('pose')
    wrapper.unmount()
  })
  it('keeps model choices separate from display visibility and filters pose capability', async () => {
    const draft = reactive({}) as Settings
    const models = [
      { id: 'boxes', tasks: ['image-to-detection'], rawCapabilities: [] },
      { id: 'pose', tasks: ['image-to-detection'], rawCapabilities: ['Pose Estimation'] },
    ] as unknown as ModelCapabilities[]
    const wrapper = mount(SettingsVisualAnalysis, { props: { draft, models } })
    await wrapper.get('button[aria-label="Pose model"]').trigger('click')
    const menu = wrapper.get('[role="listbox"][aria-label="Pose model"]')
    expect(menu.text()).toContain('pose')
    expect(menu.text()).not.toContain('boxes')
    await menu.findAll('[role="option"]')[1]!.trigger('click')
    expect(draft.visualAnalysisModels?.structure).toBe('pose')
    expect(draft.visualAnalysis).toBeUndefined()
    expect(wrapper.text()).toContain('screen overlay use these choices')
    wrapper.unmount()
  })
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