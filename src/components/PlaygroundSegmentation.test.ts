import { mount } from '@vue/test-utils'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import { classColor } from '@/config/segmentation'
import PlaygroundSegmentation from './PlaygroundSegmentation.vue'

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

it('colors Infer class IDs and shows a matching legend over the source', async () => {
  let loaded!: () => void
  class MockImage {
    naturalWidth = 2
    naturalHeight = 1
    onload: (() => void) | null = null
    onerror: (() => void) | null = null
    set src(_value: string) {
      loaded = () => this.onload?.()
    }
  }
  vi.stubGlobal('Image', MockImage)
  const pixels = { data: new Uint8ClampedArray([0, 0, 0, 255, 12, 12, 12, 255]) }
  const putImageData = vi.fn<() => void>()
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    drawImage: vi.fn<() => void>(),
    getImageData: () => pixels,
    putImageData,
  } as unknown as CanvasRenderingContext2D)
  const wrapper = mount(PlaygroundSegmentation, {
    props: {
      image: 'source',
      classMap: 'map',
      classes: [
        { id: 0, label: 'wall' },
        { id: 12, label: 'person' },
      ],
    },
  })
  loaded()
  await wrapper.vm.$nextTick()
  expect(Array.from(pixels.data)).toEqual([...classColor(0), 255, ...classColor(12), 255])
  expect(putImageData).toHaveBeenCalledOnce()
  expect(wrapper.get('[aria-label="Segmentation legend"]').text()).toContain('person')
  expect(wrapper.get('[alt="Source image"]').attributes('src')).toBe('source')
  await wrapper.get('[aria-label="Segmentation overlay opacity"]').setValue('25')
  expect(wrapper.get('canvas').attributes('style')).toContain('0.25')
  wrapper.unmount()
})