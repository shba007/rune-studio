import { effectScope } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import { useWhen } from './useWhen'
afterEach(() => vi.useRealTimers())
it('labels future dates correctly and uses readable dates across years', () => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date(2026, 9, 27, 10))
  const scope = effectScope()
  scope.run(() => {
    expect(useWhen(new Date(2026, 9, 27, 9).toISOString()).label.value).toBe('Today, 9:00 AM')
    expect(useWhen(new Date(2026, 9, 28, 7).toISOString()).label.value).toBe('Tomorrow, 7:00 AM')
    expect(useWhen(new Date(2026, 9, 26, 9).toISOString()).label.value).toBe('Yesterday, 9:00 AM')
    expect(useWhen(new Date(2027, 0, 2, 7).toISOString()).label.value).toBe('2 Jan 2027, 7:00 AM')
    expect(useWhen('invalid').valid.value).toBe(false)
  })
  scope.stop()
})
it('updates the calendar label when the device crosses midnight', () => {
  vi.useFakeTimers()
  vi.setSystemTime(new Date(2026, 9, 27, 23, 59, 30))
  const scope = effectScope()
  const when = scope.run(() => useWhen(new Date(2026, 9, 28, 7).toISOString()))!
  expect(when.label.value).toBe('Tomorrow, 7:00 AM')
  vi.advanceTimersByTime(60_000)
  expect(when.label.value).toBe('Today, 7:00 AM')
  scope.stop()
})