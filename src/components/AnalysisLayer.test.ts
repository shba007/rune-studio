import { mount } from '@vue/test-utils'
import { expect, it } from 'vite-plus/test'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import AnalysisLayer from './AnalysisLayer.vue'

it('places captured-window findings at the window origin and labels them as observations', () => {
  const wrapper = mount(AnalysisLayer, {
    props: {
      snapshot: {
        ...EMPTY_SNAPSHOT,
        id: 'insight-test',
        status: 'done',
        monitor: { id: 1, x: 200, y: 100, width: 800, height: 600, scaleFactor: 1, primary: false },
        findings: [
          {
            n: 1,
            label: 'Error',
            kind: 'issue',
            box: [0, 0, 500, 500],
            point: null,
            description: '',
            evidence: '',
          },
        ],
      },
      view: { width: 1920, height: 1080, scaleFactor: 1, originX: 0, originY: 0 },
      scanKey: 0,
    },
  })
  const marker = wrapper.get('[data-marker="1"]')
  expect(marker.attributes('style')).toContain('left: 200px')
  expect(marker.attributes('style')).toContain('top: 100px')
  expect(marker.attributes('style')).toContain('width: 400px')
  expect(marker.text()).toContain('Error')
  expect(wrapper.text()).toContain('Captured observations')
  wrapper.unmount()
})