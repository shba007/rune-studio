import { describe, expect, it } from 'vite-plus/test'

import { friendly, noticeMillis } from './errors'

describe('friendly', () => {
  it('gives an actionable next step when Screen Insight inference times out', () => {
    expect(friendly('the model did not answer within 60 seconds')).toBe(
      'The Analysis model took too long. Retry, or check Infer for a busy or loading model.',
    )
  })

  it('keeps a short plain message as it is', () => {
    expect(friendly('A task is already running')).toBe('A task is already running')
    expect(friendly('Error: Stop the current task before listening')).toBe(
      'Stop the current task before listening',
    )
  })

  it('says the server cannot be reached in one sentence', () => {
    expect(
      friendly('could not reach the model: error sending request for url (http://x/v1/chat)'),
    ).toBe("Can't reach the model server. Is it running?")
  })

  it('does not show the JSON body of a server error', () => {
    const raw = 'the model server answered 400: {"error":{"message":"llama-server error: {...}"}}'
    expect(friendly(raw)).toBe('The model server answered 400. Try again, or check its log.')
    expect(friendly('could not parse: {"a":1}')).toBe('could not parse')
  })

  it('names a rejected key', () => {
    expect(friendly('the model server answered 401: unauthorized')).toBe(
      'The model server rejected the API key.',
    )
  })

  it('cuts a very long message', () => {
    const long = 'x'.repeat(400)
    expect(friendly(long).length).toBeLessThanOrEqual(140)
    expect(friendly(long).endsWith('…')).toBe(true)
  })
})

describe('noticeMillis', () => {
  it('gives a short notice four seconds and a long one up to twelve', () => {
    expect(noticeMillis('Stopped')).toBe(4000)
    expect(noticeMillis('x'.repeat(100))).toBe(5500)
    expect(noticeMillis('x'.repeat(1000))).toBe(12000)
  })
})