import { describe, expect, it } from 'vite-plus/test'

import { dotsLeft, readOffer, reviewMeta } from './review'

describe('the review card helpers', () => {
  it('words the duration and the step count', () => {
    expect(reviewMeta(55, 6)).toBe('0:55 · 6 STEPS')
    expect(reviewMeta(125, 1)).toBe('2:05 · 1 STEP')
    expect(reviewMeta(-3, 0)).toBe('0:00 · 0 STEPS')
  })

  it('counts the dots down over 15 seconds and never below zero', () => {
    expect(dotsLeft(0, 15_000)).toBe(15)
    expect(dotsLeft(5_000, 15_000)).toBe(10)
    expect(dotsLeft(14_500, 15_000)).toBe(1)
    expect(dotsLeft(15_000, 15_000)).toBe(0)
    expect(dotsLeft(99_000, 15_000)).toBe(0)
    expect(dotsLeft(0, 99_000)).toBe(15)
  })

  it('reads an offer and rejects garbage', () => {
    expect(
      readOffer({
        runId: 'a',
        task: 't',
        outcome: 'succeeded',
        steps: 6,
        durationSecs: 55,
        source: 'Claude',
      }),
    ).toEqual({
      runId: 'a',
      task: 't',
      outcome: 'succeeded',
      steps: 6,
      durationSecs: 55,
      source: 'Claude',
    })
    expect(readOffer({ runId: 'a', outcome: 'weird' })?.outcome).toBe('failed')
    expect(readOffer({ runId: '' })).toBeNull()
    expect(readOffer(null)).toBeNull()
    expect(readOffer('x')).toBeNull()
  })
})