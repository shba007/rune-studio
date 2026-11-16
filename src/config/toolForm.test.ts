import { expect, it } from 'vite-plus/test'
import { toolArguments, toolForm, toolResultText } from './toolForm'

it('maps primitive and enum properties with required labels and typed arguments', () => {
  const form = toolForm({
    type: 'object',
    required: ['zone', 'count', 'enabled'],
    properties: {
      zone: { type: 'string', title: 'Timezone', description: 'IANA zone' },
      count: { type: 'integer' },
      amount: { type: 'number' },
      enabled: { type: 'boolean' },
      mode: { enum: ['short', 'long'] },
      limit: { type: 'integer', enum: [0, 5] },
      optional: { type: 'string' },
    },
  })
  expect(form.raw).toBe(false)
  if (form.raw) throw new Error('Expected fields')
  expect(form.fields[0]).toMatchObject({
    name: 'zone',
    label: 'Timezone',
    description: 'IANA zone',
    required: true,
  })
  expect(form.fields.at(-1)?.required).toBe(false)
  expect(
    toolArguments(
      form,
      { zone: 'Asia/Kolkata', count: '0', amount: '1.25', enabled: 'false', mode: '1', limit: '0' },
      '',
    ),
  ).toEqual({
    zone: 'Asia/Kolkata',
    count: 0,
    amount: 1.25,
    enabled: false,
    mode: 'long',
    limit: 0,
  })
  expect(() => toolArguments(form, { zone: 'UTC', count: '1.2' }, '')).toThrow('integer')
  expect(() => toolArguments(form, {}, '')).toThrow('required')
})

it('falls back to one raw object for nested, union, reference and malformed schemas', () => {
  for (const property of [
    { type: 'object' },
    { type: 'array', items: { type: 'string' } },
    { anyOf: [{ type: 'string' }, { type: 'number' }] },
    { type: ['string', 'null'] },
    { $ref: '#/$defs/value' },
    { type: 'string', const: 'fixed' },
  ]) {
    expect(
      toolForm({ type: 'object', properties: { simple: { type: 'string' }, complex: property } }),
    ).toEqual({ raw: true })
  }
  expect(toolForm({ type: 'object', anyOf: [] })).toEqual({ raw: true })
  expect(toolForm({ type: 'object', additionalProperties: { type: 'number' } })).toEqual({
    raw: true,
  })
  expect(toolForm(null)).toEqual({ raw: true })
  expect(toolForm({ type: 'object', required: ['missing'] })).toEqual({ raw: true })
  expect(toolArguments({ raw: true }, {}, '{"nested":{"values":[1,true]}}')).toEqual({
    nested: { values: [1, true] },
  })
  expect(() => toolArguments({ raw: true }, {}, '[]')).toThrow('JSON object')
  expect(() => toolArguments({ raw: true }, {}, '{')).toThrow(/JSON/)
})

it('preserves result text and only pretty-prints valid JSON', () => {
  expect(toolResultText('plain text\n')).toBe('plain text\n')
  expect(toolResultText('{"value":1}')).toBe('{\n  "value": 1\n}')
})