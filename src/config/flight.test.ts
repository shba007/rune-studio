import { describe, expect, it } from 'vite-plus/test'

import {
  actionLayer,
  argsText,
  caption,
  clock,
  eventsOfStep,
  faster,
  guardText,
  hiddenText,
  markerOf,
  nextTab,
  phaseTimes,
  segments,
  slower,
  stepAtTime,
  stepBadge,
  stepStarts,
  tabsFor,
  timeOfStep,
  totalMs,
  visibleRows,
  type FlightStep,
} from './flight'

function step(n: number, at: string | null, extra: Partial<FlightStep> = {}): FlightStep {
  return {
    step: n,
    startedUtc: at,
    goal: null,
    tool: 'click',
    argsJson: '{"x":100,"y":200}',
    reason: 'Open the menu',
    riskStated: null,
    riskFinal: null,
    riskReasonsJson: null,
    gate: null,
    result: 'ok',
    error: null,
    observationPath: null,
    observationFile: null,
    promptText: null,
    thinkingChars: null,
    completionTokens: null,
    phases: {
      observe: 0,
      screenCheck: null,
      finishCheck: null,
      decide: null,
      gate: null,
      act: null,
      settle: null,
    },
    ...extra,
  }
}

const START = '2026-10-20T10:00:00Z'

describe('time and step mapping', () => {
  it('moves the first step to 0 and keeps the others in order', () => {
    const steps = [
      step(1, '2026-10-20T10:00:02Z'),
      step(2, '2026-10-20T10:00:05Z'),
      step(3, '2026-10-20T10:00:09Z'),
    ]
    expect(stepStarts(START, steps)).toEqual([0, 3000, 7000])
  })

  it('adds the observe time and never goes backwards', () => {
    const steps = [
      step(1, '2026-10-20T10:00:00Z'),
      step(2, '2026-10-20T10:00:03Z', { phases: { ...step(2, null).phases, observe: 400 } }),
      step(3, '2026-10-20T10:00:01Z'),
    ]
    expect(stepStarts(START, steps)).toEqual([0, 3400, 3400])
  })

  it('holds steps that share a second at the same time, and a session under a second', () => {
    const same = [step(1, START), step(2, START), step(3, START)]
    expect(stepStarts(START, same)).toEqual([0, 0, 0])
    expect(stepAtTime([0, 0, 0], 0)).toBe(2)
    expect(stepAtTime([0, 0, 0], 500)).toBe(2)
  })

  it('finds the step that has begun at a time', () => {
    const starts = [0, 3000, 7000]
    expect(stepAtTime(starts, 0)).toBe(0)
    expect(stepAtTime(starts, 2999)).toBe(0)
    expect(stepAtTime(starts, 3000)).toBe(1)
    expect(stepAtTime(starts, 99_000)).toBe(2)
    expect(stepAtTime(starts, -5)).toBe(0)
    expect(stepAtTime([], 100)).toBe(-1)
  })

  it('round-trips a step through its time and clamps out-of-range steps', () => {
    const starts = [0, 3000, 7000]
    for (const index of [0, 1, 2]) expect(stepAtTime(starts, timeOfStep(starts, index))).toBe(index)
    expect(timeOfStep(starts, 9)).toBe(7000)
    expect(timeOfStep(starts, -1)).toBe(0)
    expect(timeOfStep([], 1)).toBe(0)
  })

  it('draws each segment as wide as the step lasted, the last one to the end', () => {
    expect(segments([0, 3000, 7000], 10_000)).toEqual([
      { index: 0, start: 0, length: 3000 },
      { index: 1, start: 3000, length: 4000 },
      { index: 2, start: 7000, length: 3000 },
    ])
    expect(totalMs([0, 3000], { durationSecs: null })).toBe(4000)
    expect(totalMs([0], { durationSecs: 12 })).toBe(12_000)
  })

  it('formats a clock', () => {
    expect(clock(0)).toBe('0:00')
    expect(clock(55_900)).toBe('0:55')
    expect(clock(3_723_000)).toBe('1:02:03')
    expect(clock(-5)).toBe('0:00')
  })

  it('sums the check times into observe', () => {
    const phases = {
      ...step(1, null).phases,
      observe: 100,
      screenCheck: 50,
      finishCheck: 25,
      decide: 700,
    }
    expect(phaseTimes(phases)[0]).toEqual({ phase: 'observe', ms: 175 })
    expect(phaseTimes(phases)[1]).toEqual({ phase: 'decide', ms: 700 })
  })
})

describe('typed text is hidden until shown', () => {
  const typing = step(2, null, {
    tool: 'type_text',
    argsJson: '{"text":"hunter2"}',
    reason: 'Type hunter2 in the box',
  })

  it('masks the arguments, the caption and the action layer', () => {
    expect(argsText(typing, false)).not.toContain('hunter2')
    expect(argsText(typing, false)).toContain('7 characters')
    expect(caption(typing, false)).not.toContain('hunter2')
    const layer = actionLayer(typing, false)
    expect(layer).toEqual({ kind: 'type', text: hiddenText('hunter2') })
  })

  it('shows them when asked', () => {
    expect(argsText(typing, true)).toContain('hunter2')
    expect(caption(typing, true)).toContain('hunter2')
    expect(actionLayer(typing, true)).toEqual({ kind: 'type', text: 'hunter2' })
  })

  it('leaves other steps alone', () => {
    const click = step(1, null)
    expect(argsText(click, false)).toBe(click.argsJson)
    expect(guardText(click, 'x', false)).toBe('x')
  })

  it('counts a single character in the singular', () => {
    expect(hiddenText('a')).toBe('•••• (1 character)')
  })
})

describe('the action layer and the caption', () => {
  it('describes clicks, scrolls and keys', () => {
    expect(actionLayer(step(1, null), true)).toEqual({
      kind: 'click',
      x: 100,
      y: 200,
      label: 'CLICK',
    })
    expect(
      actionLayer(step(1, null, { argsJson: '{"x":1,"y":2,"double":true}' }), true),
    ).toMatchObject({ label: 'DOUBLE-CLICK' })
    expect(
      actionLayer(
        step(1, null, { tool: 'scroll', argsJson: '{"x":5,"y":6,"direction":"down"}' }),
        true,
      ),
    ).toMatchObject({ kind: 'scroll', label: 'SCROLL DOWN' })
    expect(
      actionLayer(step(1, null, { tool: 'press_keys', argsJson: '{"keys":["ctrl","s"]}' }), true),
    ).toEqual({ kind: 'keys', text: 'ctrl + s' })
  })

  it('draws nothing for what it cannot place', () => {
    expect(actionLayer(undefined, true)).toEqual({ kind: 'none' })
    expect(actionLayer(step(1, null, { argsJson: 'not json' }), true)).toEqual({ kind: 'none' })
    expect(actionLayer(step(1, null, { tool: 'wait' }), true)).toEqual({ kind: 'none' })
  })

  it('words the step badge and the caption', () => {
    expect(stepBadge(2, 6, step(3, null, { tool: 'press_keys' }))).toBe('STEP 3 / 6 · PRESS KEYS')
    expect(caption(step(1, null), true)).toBe('Open the menu')
    expect(caption(step(1, null, { error: 'no such window', result: 'failed' }), true)).toBe(
      'Open the menu Failed: no such window',
    )
    expect(caption(undefined, true)).toBe('')
  })
})

describe('drawer tabs show only where there is data', () => {
  const bare = step(1, null, { reason: null })

  it('lists Step alone for a bare step', () => {
    expect(tabsFor({ step: bare, calls: [], hasFiles: false, toolEvents: 0 })).toEqual(['step'])
    expect(tabsFor({ step: undefined, calls: [], hasFiles: true, toolEvents: 3 })).toEqual([])
  })

  it('adds a tab for each kind of data present', () => {
    const rich = step(1, null, {
      thinkingChars: 120,
      promptText: 'hello',
      gate: 'allowed',
      phases: { ...bare.phases, decide: 900 },
    })
    const calls = [{ id: 1 }] as never
    expect(tabsFor({ step: rich, calls, hasFiles: true, toolEvents: 1 })).toEqual([
      'step',
      'reasoning',
      'prompt',
      'safety',
      'api',
      'timing',
      'tools',
      'files',
    ])
  })

  it('cycles through the tabs that exist', () => {
    expect(nextTab(['step', 'api'], 'step')).toBe('api')
    expect(nextTab(['step', 'api'], 'api')).toBe('step')
    expect(nextTab(['step'], 'timing')).toBe('step')
    expect(nextTab([], 'step')).toBeNull()
  })
})

describe('long lists and events', () => {
  it('draws only the rows in view plus a margin', () => {
    expect(visibleRows(200, 0, 100, 20)).toEqual({ from: 0, to: 9 })
    expect(visibleRows(200, 2000, 100, 20)).toEqual({ from: 96, to: 109 })
    expect(visibleRows(5, 0, 1000, 20)).toEqual({ from: 0, to: 5 })
    expect(visibleRows(0, 0, 100, 20)).toEqual({ from: 0, to: 0 })
    const { from, to } = visibleRows(200, 1000, 120, 24)
    expect(to - from).toBeLessThan(20)
  })

  it('puts an event in the step that was current', () => {
    const steps = [step(1, '2026-10-20T10:00:01Z'), step(2, '2026-10-20T10:00:05Z')]
    const events = [
      { id: 1, atUtc: '2026-10-20T10:00:02Z', kind: 'steer', dataJson: null },
      { id: 2, atUtc: '2026-10-20T10:00:06Z', kind: 'ask_user', dataJson: null },
    ]
    expect(eventsOfStep(events, START, steps, 0).map((e) => e.id)).toEqual([1])
    expect(eventsOfStep(events, START, steps, 1).map((e) => e.id)).toEqual([2])
    expect(eventsOfStep(events, START, steps, 5)).toEqual([])
  })

  it('names the markers and marks only errors as errors', () => {
    expect(markerOf({ id: 1, atUtc: null, kind: 'invalid_answer', dataJson: null })).toEqual({
      word: 'INVALID',
      error: true,
    })
    expect(markerOf({ id: 1, atUtc: null, kind: 'steer', dataJson: null })?.error).toBe(false)
    expect(markerOf({ id: 1, atUtc: null, kind: 'note', dataJson: null })).toBeNull()
  })

  it('steps through the speeds', () => {
    expect(faster(1)).toBe(2)
    expect(faster(4)).toBe(4)
    expect(slower(1)).toBe(0.5)
    expect(slower(0.5)).toBe(0.5)
  })
})

describe('the picture and the track', () => {
  it('fits a box inside a space without changing its shape', async () => {
    const { fitBox } = await import('./flight')
    expect(fitBox(1000, 500, 16 / 9)).toEqual({ width: 888, height: 500 })
    expect(fitBox(400, 900, 16 / 9)).toEqual({ width: 400, height: 225 })
    expect(fitBox(0, 500, 1.5)).toEqual({ width: 0, height: 0 })
  })

  it('draws only the segments near the visible part of a long run', async () => {
    const { visibleSegments, segments: build } = await import('./flight')
    const starts = Array.from({ length: 200 }, (_, index) => index * 1000)
    const all = build(starts, 200_000)
    const shown = visibleSegments(all, 0.1, 5000, 600)
    expect(shown.length).toBeLessThan(15)
    expect(shown.length).toBeGreaterThan(0)
    expect(visibleSegments(all, 0, 0, 600)).toEqual([])
    expect(visibleSegments(all, 0.001, 0, 600).length).toBe(200)
  })

  it('turns a pixel into a time and clamps it', async () => {
    const { timeAtPixel } = await import('./flight')
    expect(timeAtPixel(100, 300, 0.1, 10_000)).toBe(4000)
    expect(timeAtPixel(-50, 0, 0.1, 10_000)).toBe(0)
    expect(timeAtPixel(5000, 0, 0.1, 10_000)).toBe(10_000)
  })

  it('zooms within limits', async () => {
    const { zoomed, ZOOM_MAX, ZOOM_MIN } = await import('./flight')
    expect(zoomed(1, 1)).toBe(ZOOM_MIN)
    expect(zoomed(ZOOM_MAX, -1)).toBe(ZOOM_MAX)
    expect(zoomed(2, -1)).toBeGreaterThan(2)
  })

  it('picks icons, finds failures and outside agents', async () => {
    const { toolIcon, failedStep, isOutsideAgent } = await import('./flight')
    expect(toolIcon('type_text')).toBe('keyboard')
    expect(toolIcon(null)).toBe('scan')
    expect(failedStep(step(1, null, { result: 'failed' }))).toBe(true)
    expect(failedStep(step(1, null))).toBe(false)
    expect(isOutsideAgent('built-in')).toBe(false)
    expect(isOutsideAgent('Claude Code')).toBe(true)
  })
})

describe('typed text across the whole record', () => {
  const steps = [
    step(1, null, { tool: 'type_text', argsJson: '{"text":"pass\\"word"}' }),
    step(2, null, { tool: 'type_text', argsJson: '{"text":"hi"}' }),
    step(3, null),
  ]

  it('collects what was typed, longest first', async () => {
    const { typedTexts } = await import('./flight')
    expect(typedTexts(steps)).toEqual(['pass"word', 'hi'])
  })

  it('masks it anywhere in a body, escaped or not, until shown', async () => {
    const { typedTexts, maskTyped } = await import('./flight')
    const typed = typedTexts(steps)
    const body = '{"history":"typed pass\\"word then hi"}'
    expect(maskTyped(body, typed, false)).not.toContain('pass')
    expect(maskTyped(body, typed, true)).toBe(body)
  })

  it('reads reasoning from a stored reply and lays out JSON', async () => {
    const { reasoningOf, pretty } = await import('./flight')
    expect(reasoningOf('{"choices":[{"message":{"reasoning_content":"think"}}]}')).toBe('think')
    expect(reasoningOf('{"choices":[{"message":{"content":"x"}}]}')).toBeNull()
    expect(reasoningOf('nope')).toBeNull()
    expect(pretty('{"a":1}')).toBe('{\n  "a": 1\n}')
    expect(pretty('plain')).toBe('plain')
  })
})