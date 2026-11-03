import { describe, expect, it } from 'vite-plus/test'
import { classColor, colorClassMap } from './segmentation'

describe('semantic class colors', () => {
  it('maps class IDs rather than treating them as visible grayscale values', () => {
    const pixels = new Uint8ClampedArray([0, 0, 0, 255, 12, 12, 12, 255, 42, 42, 42, 255])
    colorClassMap(pixels, new Set([0, 12]))
    expect(Array.from(pixels.slice(0, 4))).toEqual([...classColor(0), 255])
    expect(Array.from(pixels.slice(4, 8))).toEqual([...classColor(12), 255])
    expect(classColor(0)).not.toEqual(classColor(12))
    expect(pixels[11]).toBe(0)
  })

  it('keeps transparent pixels transparent', () => {
    const pixels = new Uint8ClampedArray([12, 12, 12, 0])
    colorClassMap(pixels, new Set([12]))
    expect(pixels[3]).toBe(0)
  })
})