import { describe, expect, it } from 'vite-plus/test'

import { EMPTY_SNAPSHOT } from './analysis'
import { checkImageFile, fitEdge } from './imageFile'
import {
  PLAYGROUND_GROUPS,
  depthModelIds,
  filterFindings,
  questionLimit,
  shapeQuestion,
  unshapeQuestion,
} from './playground'
import type { Finding } from '@/types/protocol'

const finding = (n: number, extra: Partial<Finding>): Finding => ({
  n,
  label: `f${n}`,
  kind: 'other',
  group: 'entities',
  box: null,
  description: '',
  evidence: '',
  point: null,
  ...extra,
})

describe('playground registry', () => {
  it('has exactly three runnable pipelines, all in Images', () => {
    const ready = PLAYGROUND_GROUPS.flatMap((g) => g.pipelines).filter((p) => p.ready)
    expect(ready.map((p) => p.id)).toEqual(['ask', 'point', 'pose', 'depth'])
    expect(PLAYGROUND_GROUPS[0]!.pipelines.slice(0, 4).map((p) => p.id)).toEqual([
      'ask',
      'point',
      'pose',
      'depth',
    ])
  })

  it('has unique pipeline ids', () => {
    const ids = PLAYGROUND_GROUPS.flatMap((g) => g.pipelines).map((p) => p.id)
    expect(new Set(ids).size).toBe(ids.length)
  })
})

describe('question shaping', () => {
  it('leaves Ask untouched and wraps Point and Pose', () => {
    expect(shapeQuestion('ask', '  Hi ')).toBe('Hi')
    expect(shapeQuestion('point', 'the save button')).toContain('Locate: the save button')
    expect(shapeQuestion('pose', '')).toContain('each visible person')
    expect(shapeQuestion('pose', 'x'.repeat(400)).length).toBeLessThanOrEqual(500)
    expect(questionLimit('ask')).toBe(500)
    expect(questionLimit('point')).toBe(400)
  })

  it('restores the typed text from a shaped question', () => {
    for (const pipeline of ['point', 'pose']) {
      expect(unshapeQuestion(pipeline, shapeQuestion(pipeline, 'the cat'))).toBe('the cat')
      expect(unshapeQuestion(pipeline, shapeQuestion(pipeline, ''))).toBe('')
    }
    expect(unshapeQuestion('ask', 'plain')).toBe('plain')
    expect(unshapeQuestion('point', 'unrelated')).toBe('unrelated')
  })
})

describe('result filtering', () => {
  const snapshot = {
    ...EMPTY_SNAPSHOT,
    findings: [
      finding(1, { box: [0, 0, 10, 10] }),
      finding(2, { point: [5, 5] }),
      finding(3, { group: 'structure', pose: { keypoints: [], connections: [] } }),
      finding(4, {}),
      finding(5, { group: 'depth', box: [1, 1, 2, 2] }),
    ],
  }

  it('keeps everything for Ask', () => {
    expect(filterFindings('ask', snapshot).findings).toHaveLength(5)
  })
  it('keeps located findings for Point', () => {
    expect(filterFindings('point', snapshot).findings.map((f) => f.n)).toEqual([1, 2, 5])
  })
  it('keeps structure findings for Pose', () => {
    expect(filterFindings('pose', snapshot).findings.map((f) => f.n)).toEqual([3])
  })
})

describe('depth models', () => {
  it('lists only models with the image-to-depth task', () => {
    expect(
      depthModelIds([
        { id: 'chat', tasks: ['chat'] },
        { id: 'depth-small', tasks: ['image-to-depth'] },
        { id: 'legacy' },
      ]),
    ).toEqual(['depth-small'])
  })
})

describe('image files', () => {
  it('accepts images under the limit only', () => {
    expect(checkImageFile({ type: 'image/png', size: 10 })).toBeNull()
    expect(checkImageFile({ type: 'text/plain', size: 10 })).not.toBeNull()
    expect(checkImageFile({ type: 'image/png', size: 26 * 1024 * 1024 })).not.toBeNull()
  })
  it('shrinks the long edge to the limit and keeps the ratio', () => {
    expect(fitEdge(8192, 4096)).toEqual({ width: 4096, height: 2048 })
    expect(fitEdge(800, 600)).toEqual({ width: 800, height: 600 })
  })
})