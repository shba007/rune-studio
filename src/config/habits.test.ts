import { expect, it } from 'vite-plus/test'

import { sameDay, todaysChips, type Habit } from './habits'

const MIN = 60_000
const now = new Date(2026, 10, 19, 15, 0, 0).getTime()
const earlier = new Date(2026, 10, 19, 8, 0, 0).getTime()
const yesterday = new Date(2026, 10, 18, 20, 0, 0).getTime()

const habit = (patch: Partial<Habit> = {}): Habit => ({
  id: 'h1',
  title: 'Drink water',
  targetMinutes: null,
  state: 'pending',
  dueMs: earlier,
  completedMs: null,
  creditedMs: 0,
  ...patch,
})

it('shows a pending plain habit as a check-in chip', () => {
  expect(todaysChips([habit()], now)).toEqual([
    { id: 'h1', title: 'Drink water', kind: 'check', done: false, label: 'Drink water' },
  ])
})

it('shows a timed habit with its progress toward the target', () => {
  const chips = todaysChips(
    [habit({ title: 'Read', targetMinutes: 30, creditedMs: 12 * MIN })],
    now,
  )
  expect(chips[0]).toMatchObject({ kind: 'timed', done: false, label: 'Read · 12/30m' })
})

it('never shows more progress than the target while the habit is open', () => {
  const chips = todaysChips(
    [habit({ title: 'Read', targetMinutes: 30, creditedMs: 50 * MIN })],
    now,
  )
  expect(chips[0]!.label).toBe('Read · 30/30m')
})

it('shows a habit checked in today as done, with a timed one at its full target', () => {
  const chips = todaysChips(
    [
      habit({ id: 'a', state: 'completed', completedMs: earlier }),
      habit({
        id: 'b',
        title: 'Read',
        targetMinutes: 30,
        state: 'completed',
        completedMs: earlier,
      }),
    ],
    now,
  )
  expect(chips.map((c) => [c.id, c.done, c.label])).toEqual([
    ['a', true, 'Drink water'],
    ['b', true, 'Read · 30/30m'],
  ])
})

it('hides habits finished on an earlier day and days that were skipped', () => {
  const chips = todaysChips(
    [
      habit({ id: 'old', state: 'completed', completedMs: yesterday }),
      habit({ id: 'skipped', state: 'skipped', completedMs: yesterday }),
      habit({ id: 'running', state: 'running' }),
    ],
    now,
  )
  expect(chips).toEqual([])
})

it('lists what is still to do before what is done, keeping the given order inside each', () => {
  const chips = todaysChips(
    [
      habit({ id: 'done1', state: 'completed', completedMs: earlier }),
      habit({ id: 'todo1' }),
      habit({ id: 'todo2' }),
    ],
    now,
  )
  expect(chips.map((c) => c.id)).toEqual(['todo1', 'todo2', 'done1'])
})

it('counts the same calendar day in local time', () => {
  expect(sameDay(earlier, now)).toBe(true)
  expect(sameDay(yesterday, now)).toBe(false)
  expect(
    sameDay(new Date(2026, 10, 19, 23, 59).getTime(), new Date(2026, 10, 20, 0, 1).getTime()),
  ).toBe(false)
})