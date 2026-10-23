import { describe, expect, it } from 'vite-plus/test'
import { screenRequestText } from './screenRequest'

describe('external screen request status', () => {
  it('distinguishes a disconnected queue from delivery and actual capture', () => {
    expect(screenRequestText({ id: 1, status: 'queued', waiting: false })).toContain(
      'rune_wait_for_request',
    )
    expect(screenRequestText({ id: 1, status: 'delivered', waiting: false })).toContain(
      'Waiting for the agent to capture',
    )
    expect(screenRequestText({ id: 1, status: 'captured', waiting: false })).toContain(
      'Read its analysis in your host',
    )
    expect(screenRequestText({ id: null, status: 'idle', waiting: true })).toContain(
      'agent is waiting',
    )
    expect(screenRequestText({ id: 1, status: 'cancelled', waiting: false })).toContain('cancelled')
    expect(screenRequestText({ id: 1, status: 'expired', waiting: false })).toContain('expired')
  })
})