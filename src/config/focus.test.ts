import { expect, it } from 'vite-plus/test'

import {
  badgeMinutes,
  clock,
  isOver,
  loggedLine,
  remainingMs,
  stepMinutes,
  type FocusBlock,
} from './focus'

const block = (patch: Partial<FocusBlock> = {}): FocusBlock => ({
  id: 1,
  title: 'Write auth tests',
  doneWhen: 'tests pass',
  plannedMs: 25 * 60_000,
  startedMs: 1_000,
  endsMs: 1_000 + 25 * 60_000,
  finishedMs: null,
  outcome: null,
  taskId: null,
  handedOffMs: null,
  ...patch,
})

it('counts down to the end time and never goes below zero', () => {
  const b = block()
  expect(remainingMs(b, 1_000)).toBe(25 * 60_000)
  expect(remainingMs(b, 1_000 + 10 * 60_000)).toBe(15 * 60_000)
  expect(remainingMs(b, b.endsMs + 5_000)).toBe(0)
  expect(isOver(b, b.endsMs - 1)).toBe(false)
  expect(isOver(b, b.endsMs)).toBe(true)
})

it('writes the clock as m:ss, and h:mm:ss from an hour up', () => {
  expect(clock(25 * 60_000)).toBe('25:00')
  expect(clock(61_000)).toBe('1:01')
  expect(clock(59_001)).toBe('1:00')
  expect(clock(0)).toBe('0:00')
  expect(clock(-5)).toBe('0:00')
  expect(clock(3_600_000)).toBe('1:00:00')
  expect(clock(2 * 3_600_000 + 5 * 60_000 + 9_000)).toBe('2:05:09')
})

it('shows whole minutes left, rounded up, on the orb', () => {
  expect(badgeMinutes(25 * 60_000)).toBe(25)
  expect(badgeMinutes(24 * 60_000 + 1)).toBe(25)
  expect(badgeMinutes(1)).toBe(1)
  expect(badgeMinutes(0)).toBe(0)
})

it('steps the length by five minutes inside 1 to 240', () => {
  expect(stepMinutes(25, 1)).toBe(30)
  expect(stepMinutes(25, -1)).toBe(20)
  expect(stepMinutes(3, -1)).toBe(1)
  expect(stepMinutes(240, 1)).toBe(240)
  expect(stepMinutes(238, 1)).toBe(240)
})

it('describes a finished block by how it ended and how long it ran', () => {
  const done = block({ outcome: 'done', finishedMs: 1_000 + 10 * 60_000 })
  expect(loggedLine(done)).toBe('Done · 10 min · Write auth tests')
  const stopped = block({ outcome: 'stopped', finishedMs: 1_000 + 10_000 })
  expect(loggedLine(stopped)).toBe('Stopped · 1 min · Write auth tests')
})
it('names each option by how it compares with the suggested one', async () => {
  const { variantLabel } = await import('./focus')
  const options = [
    { title: 'a', doneWhen: '', minutes: 45 },
    { title: 'b', doneWhen: '', minutes: 20 },
    { title: 'c', doneWhen: '', minutes: 75 },
    { title: 'd', doneWhen: '', minutes: 45 },
  ]
  expect([0, 1, 2, 3].map((i) => variantLabel(options, i))).toEqual([
    'Suggested',
    'Shorter',
    'Longer',
    'Alternative',
  ])
})
const meetingAt = (startMin: number, lengthMin = 30) => ({
  id: 1,
  title: 'Meeting',
  startMs: startMin * 60_000,
  endMs: (startMin + lengthMin) * 60_000,
  source: 'manual' as const,
})

it('computes free time like the backend: gap minus a five minute buffer', async () => {
  const { freeMs } = await import('./focus')
  expect(freeMs([], 0)).toBeNull()
  expect(freeMs([meetingAt(40)], 0)).toBe(35 * 60_000)
  expect(freeMs([meetingAt(40)], 38 * 60_000)).toBe(0)
  expect(freeMs([meetingAt(40)], 50 * 60_000)).toBe(0)
  expect(freeMs([meetingAt(40)], 100 * 60_000)).toBeNull()
})

it('asks the block to wrap up from five minutes before a commitment', async () => {
  const { wrapUpFor } = await import('./focus')
  expect(wrapUpFor([meetingAt(40)], 34 * 60_000)).toBeUndefined()
  expect(wrapUpFor([meetingAt(40)], 35 * 60_000)?.title).toBe('Meeting')
  expect(wrapUpFor([meetingAt(40)], 80 * 60_000)).toBeUndefined()
})

it('writes durations rounded down', async () => {
  const { durationLabel } = await import('./focus')
  expect(durationLabel(52 * 60_000 + 59_000)).toBe('52m')
  expect(durationLabel(65 * 60_000)).toBe('1h 5m')
  expect(durationLabel(120 * 60_000)).toBe('2h')
  expect(durationLabel(0)).toBe('0m')
})

it('turns a clock time into today, or tomorrow when it has passed', async () => {
  const { startAt } = await import('./focus')
  const now = new Date(2026, 10, 19, 14, 0, 0)
  expect(new Date(startAt('15:30', now)!).getDate()).toBe(19)
  expect(new Date(startAt('15:30', now)!).getHours()).toBe(15)
  expect(new Date(startAt('09:05', now)!).getDate()).toBe(20)
  expect(new Date(startAt('14:00', now)!).getDate()).toBe(20)
  expect(startAt('25:00', now)).toBeNull()
  expect(startAt('noon', now)).toBeNull()
  expect(startAt('', now)).toBeNull()
})
const student = {
  id: 'student',
  label: 'Student',
  summary: '',
  defaultMinutes: 25,
  capMinutes: 50,
  bufferMinutes: 10,
  wrapUpMinutes: 10,
  extendMinutes: 5,
  doneWhenLabel: 'What I will finish',
}

it('reads the buffer and the wrap-up time from the preset', async () => {
  const { freeMs, wrapUpFor } = await import('./focus')
  expect(freeMs([meetingAt(40)], 0, student)).toBe(30 * 60_000)
  expect(wrapUpFor([meetingAt(40)], 29 * 60_000, student)).toBeUndefined()
  expect(wrapUpFor([meetingAt(40)], 30 * 60_000, student)?.title).toBe('Meeting')
})

it('stops the length stepper at the preset cap', async () => {
  const { stepMinutes } = await import('./focus')
  expect(stepMinutes(45, 1, 50)).toBe(50)
  expect(stepMinutes(50, 1, 50)).toBe(50)
  expect(stepMinutes(5, -1, 50)).toBe(1)
})