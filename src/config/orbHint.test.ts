import { expect, it } from 'vite-plus/test'

import { orbHint } from './orbHint'

const idle = { updating: false, working: false, ambient: false }

it('names what a click on the orb will do', () => {
  expect(orbHint(idle)).toEqual({ action: 'close', label: 'Close' })
  expect(orbHint({ ...idle, ambient: true })).toEqual({ action: 'open', label: 'Open' })
  expect(orbHint({ ...idle, working: true })).toEqual({ action: 'stop', label: 'Stop' })
})

it('lets stop win over open and an update win over everything', () => {
  expect(orbHint({ ...idle, working: true, ambient: true }).action).toBe('stop')
  expect(orbHint({ updating: true, working: true, ambient: true })).toEqual({
    action: 'busy',
    label: 'Updating',
  })
})