import { describe, expect, it } from 'vite-plus/test'

import { lengthText, toneOf } from './artifacts'

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