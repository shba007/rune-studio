import { expect, it } from 'vite-plus/test'
import { parseResult, resultContent, resultLabel } from './toolResult'

it('decodes MCP text JSON while retaining all content blocks and ordinary text', () => {
  const text = JSON.stringify({
    content: [
      { type: 'text', text: '{"result":{"timezone":"Asia/Kolkata","utcOffset":"+05:30"}}' },
      { type: 'text', text: 'A plain note\nwith another line' },
    ],
    isError: false,
    _meta: { server: 'Kit' },
  })
  expect(resultContent(text)).toEqual([
    { result: { timezone: 'Asia/Kolkata', utcOffset: '+05:30' } },
    'A plain note\nwith another line',
  ])
  expect(resultContent('{"content":[],"resultType":"complete"}')).toEqual([
    { content: [], resultType: 'complete' },
  ])
})

it('preserves values without interpreting ordinary string fields', () => {
  expect(parseResult('false')).toBe(false)
  expect(resultContent('{"text":"123","value":0,"ok":false,"empty":null}')).toEqual([
    { text: '123', value: 0, ok: false, empty: null },
  ])
  expect(resultContent('not JSON {')).toEqual(['not JSON {'])
  expect(resultLabel('timestampEpochSeconds')).toBe('Timestamp Epoch Seconds')
  expect(resultLabel('file_name')).toBe('File name')
})