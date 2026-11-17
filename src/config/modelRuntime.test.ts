import { expect, it } from 'vite-plus/test'
import {
  applyRuntime,
  maxContext,
  readRuntime,
  validateRuntime,
  type Catalog,
  type RuntimeDraft,
} from './modelRuntime'

const holo = () => ({
  id: 'holo',
  name: 'Holo',
  modality: 'Text',
  max_context_length: 32768,
  local: { path: 'C:/m.gguf', context_length: -1, gpu_layers: 99, args: '-fa on' },
  sampling: { temperature: 0, max_tokens: 512 },
  extra: { mmproj_path: 'C:/mm.gguf' },
})
const catalog = (): Catalog => ({
  models: [holo(), { id: 'cloud', name: 'Cloud', remote: { provider: 'x' } }],
  roles: { agent: 'holo' },
})
const draftOf = (over: Partial<RuntimeDraft> = {}): RuntimeDraft => ({
  ...readRuntime(holo()),
  ...over,
})

it('reads auto context, and a manual number, and the sampling fields as text', () => {
  expect(readRuntime(holo())).toEqual({
    contextMode: 'auto',
    contextLength: '',
    temperature: '0',
    topP: '',
    topK: '',
    minP: '',
    maxTokens: '512',
    reasoningEffort: '',
  })
  const manual = { ...holo(), local: { path: 'p', context_length: 8192 } }
  expect(readRuntime(manual)).toMatchObject({ contextMode: 'manual', contextLength: '8192' })
  expect(maxContext(holo())).toBe(32768)
  expect(maxContext({ id: 'x', name: 'x' })).toBeNull()
})

it('writes manual context and sampling and leaves every other field untouched', () => {
  const updated = applyRuntime(
    catalog(),
    'holo',
    draftOf({ contextMode: 'manual', contextLength: '8192', maxTokens: '2048', topK: '40' }),
  )
  const model = updated.models[0]!
  expect(model.local).toEqual({
    path: 'C:/m.gguf',
    context_length: 8192,
    gpu_layers: 99,
    args: '-fa on',
  })
  expect(model.sampling).toEqual({ temperature: 0, max_tokens: 2048, top_k: 40 })
  expect(model.extra).toEqual({ mmproj_path: 'C:/mm.gguf' })
  expect(updated.models[1]).toEqual(catalog().models[1])
  expect(updated.roles).toEqual({ agent: 'holo' })
})

it('auto writes -1, and an empty field removes its key instead of storing zero', () => {
  const manual = {
    ...catalog(),
    models: [{ ...holo(), local: { path: 'p', context_length: 4096 } }],
  }
  const updated = applyRuntime(manual, 'holo', draftOf({ temperature: '', maxTokens: '' }))
  expect(updated.models[0]!.local).toEqual({ path: 'p', context_length: -1 })
  expect(updated.models[0]!.sampling).toBeUndefined()
})

it('does not add a local block to a remote model but still writes its sampling', () => {
  const updated = applyRuntime(catalog(), 'cloud', {
    ...draftOf(),
    temperature: '0.4',
    maxTokens: '',
  })
  const cloud = updated.models[1]!
  expect(cloud.local).toBeUndefined()
  expect(cloud.sampling).toEqual({ temperature: 0.4 })
})

it('does not mutate the catalog it was given', () => {
  const original = catalog()
  const before = JSON.stringify(original)
  applyRuntime(original, 'holo', draftOf({ contextMode: 'manual', contextLength: '1024' }))
  expect(JSON.stringify(original)).toBe(before)
})

it('accepts valid values and empty optional fields', () => {
  expect(validateRuntime(draftOf(), 32768)).toEqual({})
  expect(
    validateRuntime(
      draftOf({
        contextMode: 'manual',
        contextLength: '32768',
        temperature: '5',
        topP: '1',
        topK: '0',
        minP: '0',
        maxTokens: '1',
        reasoningEffort: 'high',
      }),
      32768,
    ),
  ).toEqual({})
})

it('rejects context outside the model range and values Infer would refuse', () => {
  const manual = (contextLength: string) =>
    validateRuntime(draftOf({ contextMode: 'manual', contextLength }), 32768).contextLength
  expect(manual('')).toMatch(/whole number/)
  expect(manual('abc')).toMatch(/whole number/)
  expect(manual('4096.5')).toMatch(/whole number/)
  expect(manual('100')).toMatch(/at least 256/)
  expect(manual('40000')).toMatch(/at most 32768/)
  expect(
    validateRuntime(draftOf({ contextMode: 'manual', contextLength: '40000' }), null).contextLength,
  ).toBeUndefined()
  const bad = validateRuntime(
    draftOf({
      temperature: '6',
      topP: '0',
      topK: '-1',
      minP: '2',
      maxTokens: '0',
      reasoningEffort: 'extreme',
    }),
    null,
  )
  expect(Object.keys(bad).sort()).toEqual(
    ['maxTokens', 'minP', 'reasoningEffort', 'temperature', 'topK', 'topP'].sort(),
  )
  expect(validateRuntime(draftOf({ temperature: 'hot' }), null).temperature).toBeDefined()
})

it('ignores the typed context number while the mode is auto', () => {
  expect(validateRuntime(draftOf({ contextMode: 'auto', contextLength: 'junk' }), 1000)).toEqual({})
})