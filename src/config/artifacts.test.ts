import { describe, expect, it } from 'vite-plus/test'

import { lengthText, timingText, toneOf } from './artifacts'

describe('lengthText', () => {
  it('reads seconds and minutes', () => {
    const at = (end: string) => ({ startedUtc: '2026-10-20T10:00:00Z', endedUtc: end })
    expect(lengthText(at('2026-10-20T10:00:45Z'))).toBe('45 s')
    expect(lengthText(at('2026-10-20T10:02:05Z'))).toBe('2 min 5 s')
    expect(lengthText(at('2026-10-20T10:03:00Z'))).toBe('3 min')
    expect(lengthText({ startedUtc: '2026-10-20T10:00:00Z', endedUtc: null })).toBe('')
  })
})

describe('toneOf', () => {
  it('maps the outcome to a tone', () => {
    expect(toneOf('succeeded')).toBe('ok')
    expect(toneOf('failed')).toBe('bad')
    expect(toneOf('stopped')).toBe('stopped')
    expect(toneOf(null)).toBe('open')
  })
})

it('formats subsecond, second, minute and hour timings for people', () => {
  expect(timingText(363)).toBe('363 ms')
  expect(timingText(5020)).toBe('5.02 s')
  expect(timingText(5700)).toBe('5.7 s')
  expect(timingText(125_000)).toBe('2 min 5 s')
  expect(timingText(3_600_000)).toBe('1 h')
  expect(timingText(-1)).toBe('0 ms')
  expect(timingText(Number.NaN)).toBe('0 ms')
})