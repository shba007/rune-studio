import { expect, it } from 'vite-plus/test'
import { snoozeMinutes, tomorrowMorningMinutes } from './alarmSound'

it('maps snooze choices to minutes, with tomorrow meaning 9 am the next day', () => {
  const now = new Date(2026, 9, 30, 22, 46, 20)
  expect(snoozeMinutes('10', now)).toBe(10)
  expect(snoozeMinutes('60', now)).toBe(60)
  expect(snoozeMinutes('tomorrow', now)).toBe(614)
  expect(snoozeMinutes('unknown', now)).toBe(10)
  expect(tomorrowMorningMinutes(new Date(2026, 9, 30, 9, 0, 0))).toBe(1440)
})