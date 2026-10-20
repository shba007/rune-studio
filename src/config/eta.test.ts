import { describe, expect, it } from 'vite-plus/test'

import { etaText, remainingMs } from './eta'

const timing = { startedAt: 0, goalStartedAt: 60_000, doneGoals: 1 }

describe('remainingMs', () => {
  it('has no estimate before a goal is done or after the last', () => {
    expect(remainingMs({ ...timing, doneGoals: 0 }, 3, 0, 1000)).toBeNull()
    expect(remainingMs(timing, 3, 3, 1000)).toBeNull()
    expect(remainingMs(timing, 0, 0, 1000)).toBeNull()
  })

  it('counts down from the average time of the finished goals', () => {
    expect(remainingMs(timing, 3, 1, 60_000)).toBe(120_000)
    expect(remainingMs(timing, 3, 1, 90_000)).toBe(90_000)
  })

  it('never goes below zero', () => {
    expect(remainingMs(timing, 2, 1, 200_000)).toBe(0)
  })
})

describe('etaText', () => {
  it('speaks in seconds, minutes and hours', () => {
    expect(etaText(0)).toBe('any moment')
    expect(etaText(3000)).toBe('any moment')
    expect(etaText(12_000)).toBe('~15 s')
    expect(etaText(90_000)).toBe('~2 min')
    expect(etaText(3_900_000)).toBe('~1 h 5 min')
    expect(etaText(7_200_000)).toBe('~2 h')
  })
})