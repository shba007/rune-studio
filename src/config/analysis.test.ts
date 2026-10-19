import { describe, expect, it } from 'vite-plus/test'

import type { Box } from '@/types/protocol'

import { layoutChips, readSnapshot, type ChipInput, type ChipLayout } from './analysis'

const stage = { width: 800, height: 450 }
const item = (n: number, box: Box, label = `Item ${n}`): ChipInput => ({ n, label, box })

function anyOverlap(chips: ChipLayout[]) {
  return chips.some((a, i) =>
    chips
      .slice(i + 1)
      .some((b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h),
  )
}

describe('layoutChips', () => {
  it('puts a chip to the right of its box with a leader from the chip to the box edge', () => {
    const [chip] = layoutChips([item(1, [100, 200, 300, 260])], stage)
    expect(chip!.side).toBe('right')
    expect(chip!.x).toBe(314)
    expect(chip!.leader).toEqual({
      x1: 314,
      y1: chip!.y + chip!.h / 2,
      x2: 300,
      y2: chip!.y + chip!.h / 2,
    })
    // The chip's height lies inside the box's, so the leader is level.
    expect(chip!.y + chip!.h / 2).toBeGreaterThan(200)
    expect(chip!.y + chip!.h / 2).toBeLessThan(260)
  })

  it('moves to the left of a box near the right edge, and inside a box that spans the width', () => {
    const [left, inside] = layoutChips(
      [item(1, [800, 100, 980, 160]), item(2, [10, 700, 990, 760], 'A very long label here')],
      stage,
    )
    expect(left!.side).toBe('left')
    expect(left!.x + left!.w).toBeCloseTo(786)
    expect(left!.leader!.x2).toBe(800)
    expect(inside!.side).toBe('inside')
    expect(inside!.leader).toBeNull()
  })

  it('stacks chips of boxes on the same row without overlap', () => {
    const items = Array.from({ length: 8 }, (_, i) => item(i + 1, [100, 400, 200, 440]))
    const chips = layoutChips(items, stage)
    expect(anyOverlap(chips)).toBe(false)
  })

  it('keeps every chip inside the picture, also at the bottom edge', () => {
    const items = Array.from({ length: 6 }, (_, i) => item(i + 1, [50, 960 + i, 150, 1000]))
    for (const chip of layoutChips(items, stage)) {
      expect(chip.y).toBeGreaterThanOrEqual(0)
      expect(chip.y + chip.h).toBeLessThanOrEqual(1000.0001)
      expect(chip.x).toBeGreaterThanOrEqual(0)
      expect(chip.x + chip.w).toBeLessThanOrEqual(1000.0001)
    }
    const chips = layoutChips(items, stage)
    expect(anyOverlap(chips)).toBe(false)
  })

  it('is deterministic and does nothing for an empty or zero-sized stage', () => {
    const items = [
      item(1, [10, 10, 90, 50]),
      item(2, [10, 20, 90, 60]),
      item(3, [500, 20, 600, 60]),
    ]
    expect(layoutChips(items, stage)).toEqual(layoutChips(items, stage))
    expect(layoutChips([], stage)).toEqual([])
    expect(layoutChips(items, { width: 0, height: 450 })).toEqual([])
  })

  it('makes chips taller in units on a short picture (a chip is the same height in pixels)', () => {
    const tall = layoutChips([item(1, [0, 0, 50, 50])], { width: 800, height: 800 })[0]!
    const flat = layoutChips([item(1, [0, 0, 50, 50])], { width: 800, height: 400 })[0]!
    expect(flat.h).toBeCloseTo(tall.h * 2)
  })
})

describe('readSnapshot', () => {
  it('reads a full snapshot', () => {
    const snapshot = readSnapshot({
      status: 'done',
      id: 'a1',
      question: 'q',
      answer: 'a',
      findings: [
        {
          n: 1,
          label: 'Save',
          kind: 'control',
          box: [1, 2, 30, 40],
          description: 'd',
          evidence: 'e',
        },
        { n: 2, label: 'Title', kind: 'region', box: null, description: '', evidence: '' },
      ],
      model: 'm',
      latencyMs: 1500,
      error: null,
      createdUtc: 't',
      image: 'data:image/jpeg;base64,AA==',
      width: 640,
      height: 360,
    })
    expect(snapshot.status).toBe('done')
    expect(snapshot.findings[0]!.box).toEqual([1, 2, 30, 40])
    expect(snapshot.findings[1]!.box).toBeNull()
    expect(snapshot.latencyMs).toBe(1500)
  })

  it('survives garbage: unknown status and kind, a bad box, missing fields, not an object', () => {
    const snapshot = readSnapshot({
      status: 'weird',
      findings: [{ kind: 'x', box: [1, 2, 3] }, 7, null],
    })
    expect(snapshot.status).toBe('idle')
    expect(snapshot.findings.map((f) => [f.n, f.kind, f.box])).toEqual([
      [1, 'other', null],
      [2, 'other', null],
      [3, 'other', null],
    ])
    expect(readSnapshot(null).status).toBe('idle')
    expect(readSnapshot('text').findings).toEqual([])
    expect(readSnapshot({ findings: 'nope' }).findings).toEqual([])
  })
})