import { expect, it } from 'vite-plus/test'
import { taskOrbState } from './taskActivity'
it('animates planning and background work without changing desktop ownership', () => {
  expect(taskOrbState('idle', [{ phase: 'planning' }])).toBe('planning')
  expect(taskOrbState('done', [{ phase: 'queued' }])).toBe('planning')
  expect(taskOrbState('failed', [{ phase: 'running' }])).toBe('planning')
  expect(taskOrbState('awaiting_confirm', [{ phase: 'planning' }])).toBe('awaiting_confirm')
  expect(taskOrbState('acting', [{ phase: 'planning' }])).toBe('acting')
  expect(taskOrbState('listening', [{ phase: 'planning' }])).toBe('listening')
  expect(taskOrbState('idle', [{ phase: 'done' }])).toBe('done')
  expect(taskOrbState('idle', [{ phase: 'failed' }])).toBe('failed')
  expect(taskOrbState('idle', [{ phase: 'cancelled' }])).toBe('stopped')
})