import { expect, it } from 'vite-plus/test'
import { annotationLayout } from './annotations'

it('separates eight clustered labels and keeps them inside a resized preview', () => {
  const rects = Array.from({ length: 8 }, () => ({ left: 390, top: 0, width: 100, height: 300 }))
  const labels = annotationLayout(rects, 800, 400)
  expect(labels.every(Boolean)).toBe(true)
  for (const [index, item] of labels.entries()) {
    const label = item!.label
    expect(label.left).toBeGreaterThanOrEqual(0)
    expect(label.top).toBeGreaterThanOrEqual(0)
    expect(label.left + label.width).toBeLessThanOrEqual(800)
    expect(label.top + label.height).toBeLessThanOrEqual(400)
    expect(item!.leader.endsWith(`${item!.anchor.x},${item!.anchor.y}`)).toBe(true)
    for (const other of labels.slice(0, index)) {
      const b = other!.label
      expect(
        label.left >= b.left + b.width ||
          label.left + label.width <= b.left ||
          label.top >= b.top + b.height ||
          label.top + label.height <= b.top,
      ).toBe(true)
    }
  }
})

it('uses a compact offset label and horizontal-diagonal-horizontal leader like the reference', () => {
  const [annotation] = annotationLayout(
    [{ left: 290, top: 24, width: 100, height: 80 }],
    500,
    160,
    ['Structure'],
  )
  expect(annotation).not.toBeNull()
  const { label, anchor, leader } = annotation!
  expect(label.left + label.width).toBeLessThan(anchor.x)
  expect(label.width).toBe(70)
  const points = leader.split(' ').map((point) => point.split(',').map(Number))
  expect(points).toHaveLength(4)
  expect(points[0]![1]).toBe(points[1]![1])
  expect(points[1]![0]).not.toBe(points[2]![0])
  expect(points[1]![1]).not.toBe(points[2]![1])
  expect(points[2]![1]).toBe(points[3]![1])
})

it('omits callouts that cannot fit instead of overlapping them', () => {
  expect(annotationLayout([{ left: 0, top: 0, width: 1, height: 1 }], 10, 10)).toEqual([null])
})