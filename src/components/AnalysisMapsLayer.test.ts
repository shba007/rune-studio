import { mount } from '@vue/test-utils'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import AnalysisMapsLayer from './AnalysisMapsLayer.vue'

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

it('positions both maps on the captured display and hides them when stale', async () => {
  let loaded!: () => void
  class MockImage {
    naturalWidth = 1
    naturalHeight = 1
    onload: (() => void) | null = null
    set src(_source: string) {
      loaded = () => this.onload?.()
    }
  }
  vi.stubGlobal('Image', MockImage)
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    drawImage: vi.fn<() => void>(),
    getImageData: () => ({ data: new Uint8ClampedArray([12, 12, 12, 255]) }),
    putImageData: vi.fn<() => void>(),
  } as unknown as CanvasRenderingContext2D)
  const snapshot = {
    ...EMPTY_SNAPSHOT,
    id: 'insight-1',
    status: 'done' as const,
    monitor: { id: 1, x: 200, y: 100, width: 800, height: 600, scaleFactor: 1, primary: true },
    visualMaps: {
      segmentation: 'data:image/png;base64,AA==',
      classes: [{ id: 12, label: 'person' }],
      depth: 'data:image/png;base64,AQ==',
      errors: [],
    },
  }
  const wrapper = mount(AnalysisMapsLayer, {
    props: {
      snapshot,
      view: { width: 1200, height: 900, scaleFactor: 1, originX: 100, originY: 0 },
    },
  })
  loaded()
  await wrapper.vm.$nextTick()
  expect(wrapper.attributes('style')).toContain('left: 100px')
  expect(wrapper.attributes('style')).toContain('width: 800px')
  expect(wrapper.get('canvas').isVisible()).toBe(true)
  expect(wrapper.get('img').attributes('src')).toBe(snapshot.visualMaps.depth)
  await wrapper.setProps({ snapshot: { ...snapshot, stale: true } })
  expect(wrapper.find('img').exists()).toBe(false)
  wrapper.unmount()
})