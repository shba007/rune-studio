import { describe, expect, it } from 'vite-plus/test'

import type { AnalysisSnapshot } from '@/types/protocol'

import {
  type CardsState,
  EMPTY_SNAPSHOT,
  badgePosition,
  boxToCss,
  drawable,
  escape,
  formatElapsed,
  markerOpacity,
  readSnapshot,
  stageWord,
  toggleOpen,
} from './analysis'

const monitor = { id: 1, x: 0, y: 0, width: 1920, height: 1080, scaleFactor: 1, primary: true }

describe('boxToCss', () => {
  it('maps 0..1000 onto the overlay window at 100 percent', () => {
    expect(boxToCss([100, 200, 300, 260], { width: 1920, height: 1080, scaleFactor: 1 })).toEqual({
      left: 192,
      top: 216,
      width: 384,
      height: 64.8,
    })
  })

  it('is right at 125 and 150 percent: the window is physical pixels over the scale', () => {
    const at125 = boxToCss([500, 500, 1000, 1000], { width: 1920, height: 1080, scaleFactor: 1.25 })
    expect(at125).toEqual({ left: 768, top: 432, width: 768, height: 432 })
    const at150 = boxToCss([0, 0, 500, 500], { width: 2880, height: 1620, scaleFactor: 1.5 })
    expect(at150).toEqual({ left: 0, top: 0, width: 960, height: 540 })
  })

  it('does not depend on where the monitor sits on the desktop', () => {
    const view = { width: 1920, height: 1080, scaleFactor: 1 }
    expect(boxToCss([250, 250, 750, 750], view)).toEqual(boxToCss([250, 250, 750, 750], view))
    expect(boxToCss([250, 250, 750, 750], view).left).toBe(480)
  })

  it('treats a missing scale as 1', () => {
    expect(boxToCss([0, 0, 1000, 1000], { width: 800, height: 600, scaleFactor: 0 })).toEqual({
      left: 0,
      top: 0,
      width: 800,
      height: 600,
    })
  })
})

describe('badgePosition', () => {
  const view = { width: 1000, height: 1000, scaleFactor: 1 }
  it('centres the badge on the top-left corner and keeps it inside the window', () => {
    expect(badgePosition({ left: 300, top: 200, width: 100, height: 100 }, view)).toEqual({
      x: 290,
      y: 190,
    })
    expect(badgePosition({ left: 0, top: 0, width: 100, height: 100 }, view)).toEqual({
      x: 0,
      y: 0,
    })
    expect(badgePosition({ left: 1000, top: 1000, width: 10, height: 10 }, view)).toEqual({
      x: 980,
      y: 980,
    })
  })
})

describe('what is drawn on the screen', () => {
  const finding = (n: number, box: [number, number, number, number] | null) => ({
    n,
    label: `F${n}`,
    kind: 'region' as const,
    box,
    description: '',
    evidence: '',
    point: null,
  })
  const snapshot = (over: Partial<AnalysisSnapshot>): AnalysisSnapshot => ({
    ...EMPTY_SNAPSHOT,
    status: 'done',
    findings: [finding(1, [10, 10, 100, 100]), finding(2, null), finding(3, [200, 200, 300, 300])],
    monitor,
    ...over,
  })

  it('draws only findings that have a box', () => {
    expect(drawable(snapshot({})).map((f) => f.n)).toEqual([1, 3])
  })

  it('draws nothing once the screen has changed: the only way the boxes go', () => {
    expect(drawable(snapshot({ stale: true }))).toEqual([])
  })

  it('has no switch that hides the boxes: nothing but staleness changes what is drawn', () => {
    const states = (['capturing', 'asking', 'done', 'error', 'cancelled', 'idle'] as const).map(
      (status) => drawable(snapshot({ status })).length,
    )
    expect(states).toEqual([2, 2, 2, 2, 2, 2])
  })

  it('draws the selected marker at full strength and the others at 35 percent', () => {
    expect(markerOpacity(1, null)).toBe(1)
    expect(markerOpacity(1, 1)).toBe(1)
    expect(markerOpacity(2, 1)).toBe(0.35)
  })
})

describe('the stage chip and the timer', () => {
  it('names the stage', () => {
    expect(stageWord('capturing')).toBe('LOOKING')
    expect(stageWord('asking')).toBe('READING')
    expect(stageWord('done')).toBe('DONE')
    expect(stageWord('cancelled')).toBe('CANCELLED')
    expect(stageWord('error')).toBe('FAILED')
    expect(stageWord('idle')).toBeNull()
  })

  it('formats elapsed time as mm:ss', () => {
    expect(formatElapsed(0)).toBe('00:00')
    expect(formatElapsed(14_300)).toBe('00:14')
    expect(formatElapsed(61_000)).toBe('01:01')
    expect(formatElapsed(-5)).toBe('00:00')
  })
})

describe('the cards', () => {
  it('opens one finding at a time and closes the open one on a second tap', () => {
    expect(toggleOpen(null, 2)).toBe(2)
    expect(toggleOpen(2, 3)).toBe(3)
    expect(toggleOpen(3, 3)).toBeNull()
  })

  it('escapes in order: the expanded finding, the selection, then the cards', () => {
    let state: CardsState = { open: 2, selected: 2, shown: true }
    state = escape(state)
    expect(state).toEqual({ open: null, selected: 2, shown: true })
    state = escape(state)
    expect(state).toEqual({ open: null, selected: null, shown: true })
    state = escape(state)
    expect(state.shown).toBe(false)
  })
})

describe('readSnapshot', () => {
  it('reads a full snapshot', () => {
    const read = readSnapshot({
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
      monitor,
      stale: true,
      boxesRejected: 2,
      selected: 1,
    })
    expect(read.status).toBe('done')
    expect(read.findings[0]!.box).toEqual([1, 2, 30, 40])
    expect(read.findings[1]!.box).toBeNull()
    expect(read.monitor?.width).toBe(1920)
    expect([read.stale, read.boxesRejected, read.selected]).toEqual([true, 2, 1])
  })

  it('survives garbage and old shapes (an image field is ignored)', () => {
    const read = readSnapshot({
      status: 'weird',
      image: 'data:x',
      findings: [{ kind: 'x', box: [1, 2, 3] }, 7, null],
    })
    expect(read.status).toBe('idle')
    expect(read.monitor).toBeNull()
    expect(read.findings.map((f) => [f.n, f.kind, f.box])).toEqual([
      [1, 'other', null],
      [2, 'other', null],
      [3, 'other', null],
    ])
    expect(readSnapshot(null).status).toBe('idle')
    expect(readSnapshot({ findings: 'nope' }).findings).toEqual([])
    expect(readSnapshot({ monitor: { width: 0, height: 5 } }).monitor).toBeNull()
  })
})