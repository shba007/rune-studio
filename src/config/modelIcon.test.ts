import { expect, it } from 'vite-plus/test'
import { modelIcon } from './modelIcon'
it('recognizes model families across aliases, namespaces, casing, and quantizations', () => {
  expect(modelIcon('QWEN3.8-27B-UD-IQ3-S')).toBe('/model/qwen.svg')
  expect(modelIcon('google/gemma-4-12b-it')).toBe('/model/gemma.svg')
  expect(modelIcon('microsoft/Phi-4')).toBe('/model/microsoft.svg')
  expect(modelIcon('openai/gpt-oss-20b')).toBe('/model/openai.svg')
  expect(modelIcon('claude-sonnet')).toBe('/model/claude.svg')
  expect(modelIcon('new-custom-model')).toBe('/model/model.svg')
})