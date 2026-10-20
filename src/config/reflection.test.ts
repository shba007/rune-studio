import { describe, expect, it } from 'vite-plus/test'

import { clock, secondsLeft } from './reflection'

describe('the reflection countdown', () => {
  it('counts down from the deadline and stops at zero', () => {
    const endsAt = 31_000
    expect(secondsLeft(1_000, endsAt)).toBe(30)
    expect(secondsLeft(1_001, endsAt)).toBe(30)
    expect(secondsLeft(15_500, endsAt)).toBe(16)
    expect(secondsLeft(30_999, endsAt)).toBe(1)
    expect(secondsLeft(31_000, endsAt)).toBe(0)
    expect(secondsLeft(99_000, endsAt)).toBe(0)
  })

  it('decreases as the clock moves, which the confirm card once failed to do', () => {
    const readings = [0, 5_000, 10_000, 29_000].map((now) => secondsLeft(now, 30_000))
    expect(readings).toEqual([30, 25, 20, 1])
  })

  it('formats as m:ss', () => {
    expect(clock(30)).toBe('0:30')
    expect(clock(5)).toBe('0:05')
    expect(clock(0)).toBe('0:00')
    expect(clock(125)).toBe('2:05')
  })
})