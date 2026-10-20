import { describe, expect, it } from 'vite-plus/test'

import { lengthText, stepDots, toneOf, whenText } from './artifacts'

describe('whenText', () => {
  const now = new Date(2026, 9, 20, 15, 0)
  it('says today, yesterday, then the date', () => {
    expect(whenText(new Date(2026, 9, 20, 9, 5).toISOString(), now)).toMatch(/^Today /)
    expect(whenText(new Date(2026, 9, 19, 9, 5).toISOString(), now)).toMatch(/^Yesterday /)
    expect(whenText(new Date(2026, 9, 1, 9, 5).toISOString(), now)).not.toMatch(/Today|Yesterday/)
  })
  it('is empty for a missing or broken time', () => {
    expect(whenText(null, now)).toBe('')
    expect(whenText('nope', now)).toBe('')
  })
})

describe('lengthText', () => {
  it('reads seconds and minutes', () => {
    const at = (end: string) => ({ startedUtc: '2026-10-20T10:00:00Z', endedUtc: end })
    expect(lengthText(at('2026-10-20T10:00:45Z'))).toBe('45s')
    expect(lengthText(at('2026-10-20T10:02:05Z'))).toBe('2m 5s')
    expect(lengthText(at('2026-10-20T10:03:00Z'))).toBe('3m')
    expect(lengthText({ startedUtc: '2026-10-20T10:00:00Z', endedUtc: null })).toBe('')
  })
})

describe('stepDots and toneOf', () => {
  it('caps the dots and marks the overflow', () => {
    expect(stepDots(4)).toEqual({ count: 4, more: false })
    expect(stepDots(30)).toEqual({ count: 8, more: true })
    expect(stepDots(-1)).toEqual({ count: 0, more: false })
  })
  it('maps the outcome to a tone', () => {
    expect(toneOf('succeeded')).toBe('ok')
    expect(toneOf('failed')).toBe('bad')
    expect(toneOf('stopped')).toBe('stopped')
    expect(toneOf(null)).toBe('open')
  })
})