import { expect, it } from 'vite-plus/test'
import { parseReminder, parseSchedule, planFromInterpretation } from './reminder'

const now = new Date(2026, 9, 30, 22, 46, 20)
const plan = (text: string) => parseReminder(text, now)!

it('rounds a relative reminder up to the next whole minute and cleans the message', () => {
  const p = plan('remind in 1 min that i have to drink water')
  expect(p.title).toBe('Drink water')
  expect(p.local).toBe('2026-10-30T22:48')
  expect(plan('Remind me in 10 minutes to call mom').local).toBe('2026-10-30T22:57')
  expect(plan('remind me in an hour to stretch').local).toBe('2026-10-30T23:47')
  expect(plan('remind me in half an hour to stretch').local).toBe('2026-10-30T23:17')
  expect(plan('remind me in two days to pay rent').local).toBe('2026-11-01T22:47')
})

it('accepts the time at the end of the sentence', () => {
  const p = plan('remind me to call the bank in 5 minutes')
  expect(p.title).toBe('Call the bank')
  expect(p.local).toBe('2026-10-30T22:52')
})

it('handles clock times and tomorrow', () => {
  expect(plan('remind me at 11pm to sleep').local).toBe('2026-10-30T23:00')
  expect(plan('remind me at 6:30 am to run').local).toBe('2026-10-31T06:30')
  expect(plan('remind me tomorrow at 9 to submit the form').local).toBe('2026-10-31T09:00')
  expect(plan('remind me to buy milk tomorrow').local).toBe('2026-10-31T09:00')
})

it('ignores ordinary requests and incomplete reminders inline', () => {
  expect(parseReminder('open notepad', now)).toBeNull()
  expect(parseReminder('remind me to buy milk', now)).toBeNull()
  expect(parseReminder('remind me in 5 minutes', now)).toBeNull()
})

it('explains what is missing for /schedule', () => {
  expect(parseSchedule('drink water', now)).toContain('Say when')
  expect(parseSchedule('', now)).toContain('Tell Rune')
  expect(parseSchedule('in 5 min', now)).toContain('Say what')
  expect(parseSchedule('today at 7am call', now)).toContain('already passed')
  const p = parseSchedule('in 5 min drink water', now)
  expect(typeof p === 'string' ? p : p.title).toBe('Drink water')
})
it('turns a model reading into a reminder, rounding delays up and rejecting the past', () => {
  const after = planFromInterpretation(
    { kind: 'after', title: 'Drink water again', delaySeconds: 50, local: '' },
    now,
  )!
  expect(after.local).toBe('2026-10-30T22:48')
  const at = planFromInterpretation(
    { kind: 'at', title: 'Call', delaySeconds: 0, local: '2026-11-02T08:15' },
    now,
  )!
  expect(at.local).toBe('2026-11-02T08:15')
  expect(
    planFromInterpretation(
      { kind: 'at', title: 'Call', delaySeconds: 0, local: '2026-10-30T22:00' },
      now,
    ),
  ).toBeNull()
  expect(
    planFromInterpretation({ kind: 'unclear', title: '', delaySeconds: 0, local: '' }, now),
  ).toBeNull()
})