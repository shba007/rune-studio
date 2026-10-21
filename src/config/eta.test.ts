import { describe, expect, it } from 'vite-plus/test'

import { clockText, remainingMs } from './eta'

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

describe('clockText', () => {
  it('counts in minutes and seconds, with hours when needed', () => {
    expect(clockText(0)).toBe('0:00')
    expect(clockText(500)).toBe('0:01')
    expect(clockText(42_000)).toBe('0:42')
    expect(clockText(125_000)).toBe('2:05')
    expect(clockText(3_725_000)).toBe('1:02:05')
    expect(clockText(-5)).toBe('0:00')
  })
})