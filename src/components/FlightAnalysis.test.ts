import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import type { AnalysisGroup, Finding } from '@/types/protocol'
import FlightAnalysis from './FlightAnalysis.vue'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: async () => '/capture.png',
  convertFileSrc: (path: string) => path,
}))
vi.mock('@vueuse/core', async (original) => ({
  ...(await original<typeof import('@vueuse/core')>()),
  useElementSize: () => ({ width: ref(800), height: ref(400) }),
}))

describe('saved visual analysis highlights', () => {
  it('scales all four groups to the capture and hides only the selected highlights', async () => {
    const groups: AnalysisGroup[] = ['entities', 'regions', 'structure', 'depth']
    const wrapper = mount(FlightAnalysis, {
      props: {
        id: 'insight-saved',
        summary: null,
        analysis: {
          answer: 'Saved answer',
          model: 'vision',
          capturedUtc: '',
          captureFile: 'capture.png',
          error: null,
          findings: groups.map<Finding>((group, index) => ({
            n: index + 1,
            label: group,
            group,
            pose:
              group === 'structure'
                ? {
                    keypoints: [
                      { label: 'shoulder', x: 250, y: 250 },
                      { label: 'elbow', x: 500, y: 500 },
                    ],
                    connections: [[0, 1]],
                    rejected: 1,
                  }
                : null,
            kind: 'region',
            box: [250, 250, 750, 750],
            point: null,
            description: `${group} observation`,
            evidence: 'Saved evidence',
          })),
        },
      },
    })
    await flushPromises()
    expect(wrapper.findAll('[data-marker]')).toHaveLength(4)
    expect(wrapper.findAll('[data-keypoint]')).toHaveLength(2)
    expect(wrapper.text()).toContain('Some pose data was invalid and omitted')
    const marker = wrapper.get('[data-marker="1"]')
    expect(marker.attributes('style')).toContain('left: 200px')
    expect(marker.attributes('style')).toContain('top: 100px')
    expect(marker.attributes('style')).toContain('width: 400px')
    await wrapper.get('input[aria-label="Depth"]').setValue(false)
    expect(wrapper.find('[data-marker="4"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('depth observation')
    await wrapper.get('input[aria-label="Depth"]').setValue(true)
    expect(wrapper.find('[data-marker="4"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('keeps older sessions without captures readable without inventing highlights', async () => {
    const wrapper = mount(FlightAnalysis, { props: { id: 'old', summary: 'Recorded answer' } })
    await flushPromises()
    expect(wrapper.text()).toContain('Recorded answer')
    expect(wrapper.text()).toContain('No capture was saved')
    expect(wrapper.findAll('[data-marker]')).toHaveLength(0)
    wrapper.unmount()
  })
})