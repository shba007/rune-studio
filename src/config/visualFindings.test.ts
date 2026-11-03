import { expect, it } from 'vite-plus/test'
import { visualSnapshot } from './visualFindings'

it('converts Infer center boxes to the standard annotation coordinates', () => {
  const snapshot = visualSnapshot(
    {
      model: 'detector',
      data: [
        { label: 'person', confidence: 0.9, box: { x: 0.5, y: 0.4, width: 0.4, height: 0.6 } },
        { label: 'edge', confidence: 0.8, box: { x: 0, y: 0, width: 0.2, height: 0.2 } },
      ],
    },
    'image',
  )
  snapshot.findings[0]!.box!.forEach((value, index) =>
    expect(value).toBeCloseTo([300, 100, 700, 700][index]!),
  )
  expect(snapshot.findings[1]!.box).toEqual([0, 0, 100, 100])
  expect(snapshot.screenshot).toBe('image')
})

it('does not invent boxes or show low-confidence pose points', () => {
  const snapshot = visualSnapshot(
    {
      model: 'pose',
      data: [
        {
          label: 'person',
          confidence: 0.9,
          box: { x: 0.5, y: 0.5, width: 0, height: 1 },
          pose: [{ name: 'nose', x: 0.5, y: 0.3, confidence: 0.2 }],
        },
      ],
    },
    'image',
  )
  expect(snapshot.findings[0]!.box).toBeNull()
  expect(snapshot.findings[0]!.pose).toBeNull()
  expect(snapshot.findings[0]!.description).toContain('nose: 20%')
})