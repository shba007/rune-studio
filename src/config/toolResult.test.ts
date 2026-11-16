import { expect, it } from 'vite-plus/test'
import { parseResult, resultContent, resultLabel, schemaView } from './toolResult'

it('maps flat objects in schema order with labels, hints and scalar values', () => {
  const view = schemaView(
    {
      type: 'object',
      properties: {
        flag: { type: 'boolean', title: 'Enabled', description: 'Whether enabled' },
        count: { type: 'integer', minimum: 10 },
        empty: { type: 'null' },
      },
    },
    { count: 0, empty: null, flag: false },
  )
  expect(view.kind).toBe('object')
  if (view.kind !== 'object') throw new Error('Expected object')
  expect(view.fields.map((field) => field.key)).toEqual(['flag', 'count', 'empty'])
  expect(view.fields[0]).toMatchObject({
    label: 'Enabled',
    hint: 'Whether enabled',
    node: { kind: 'value', value: false },
  })
  expect(view.fields[1]?.node).toEqual({ kind: 'value', value: 0 })
  expect(view.fields[2]?.node).toEqual({ kind: 'value', value: null })
})

it('maps nested objects, scalar lists and object tables using schema columns', () => {
  const nested = schemaView(
    {
      type: 'object',
      properties: { child: { type: 'object', properties: { text: { type: 'string' } } } },
    },
    { child: { text: 'hello' } },
  )
  expect(nested).toMatchObject({
    kind: 'object',
    fields: [{ node: { kind: 'object', fields: [{ node: { kind: 'value', value: 'hello' } }] } }],
  })
  expect(schemaView({ type: 'array', items: { type: 'number' } }, [0, 1.25])).toEqual({
    kind: 'list',
    items: [
      { kind: 'value', value: 0 },
      { kind: 'value', value: 1.25 },
    ],
  })
  const table = schemaView(
    {
      type: 'array',
      items: {
        type: 'object',
        properties: { z: { type: 'string', title: 'Name' }, a: { type: 'integer' } },
      },
    },
    [
      { a: 1, z: 'one' },
      { z: 'two', extra: true },
    ],
  )
  expect(table.kind).toBe('table')
  if (table.kind !== 'table') throw new Error('Expected table')
  expect(table.columns.map((column) => column.key)).toEqual(['z', 'a', 'extra'])
  expect(table.columns[0]?.label).toBe('Name')
  expect(table.rows[1]).toEqual([
    { kind: 'value', value: 'two' },
    { kind: 'missing' },
    { kind: 'json', value: true },
  ])
})

it('keeps undescribed values and unsupported shapes as JSON without dropping data', () => {
  expect(
    schemaView(
      { type: 'object', properties: { name: { type: 'string' } } },
      { name: 'hello', unknown: { secret: 42 } },
    ),
  ).toMatchObject({
    fields: [{ key: 'name' }, { key: 'unknown', node: { kind: 'json', value: { secret: 42 } } }],
  })
  for (const schema of [
    { anyOf: [{ type: 'string' }] },
    { type: ['string', 'null'] },
    { $ref: '#/thing' },
    { type: 'array', items: { type: 'array' } },
  ]) {
    expect(schemaView(schema, { keep: [1, 2] })).toEqual({ kind: 'json', value: { keep: [1, 2] } })
  }
  expect(schemaView({ type: 'integer' }, 'not an integer')).toEqual({
    kind: 'json',
    value: 'not an integer',
  })
})

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