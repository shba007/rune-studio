export type CatalogModel = Record<string, unknown> & { id: string; name: string }
export type Catalog = {
  models: CatalogModel[]
  roles?: Record<string, string>
  [key: string]: unknown
}

export const REASONING_EFFORTS = ['none', 'low', 'medium', 'high'] as const
export const MIN_CONTEXT = 256

export interface RuntimeDraft {
  contextMode: 'auto' | 'manual'
  contextLength: string
  temperature: string
  topP: string
  topK: string
  minP: string
  maxTokens: string
  reasoningEffort: string
}

type Field = 'contextLength' | 'temperature' | 'topP' | 'topK' | 'minP' | 'maxTokens'

const SAMPLING: { field: Field; key: string }[] = [
  { field: 'temperature', key: 'temperature' },
  { field: 'topP', key: 'top_p' },
  { field: 'topK', key: 'top_k' },
  { field: 'minP', key: 'min_p' },
  { field: 'maxTokens', key: 'max_tokens' },
]

const object = (value: unknown): Record<string, unknown> =>
  value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {}
const text = (value: unknown) => (typeof value === 'number' ? String(value) : '')

export const isLocal = (model: CatalogModel) => !!model.local && typeof model.local === 'object'

export function maxContext(model: CatalogModel): number | null {
  const value = model.max_context_length
  return typeof value === 'number' && value > 0 ? value : null
}

export function readRuntime(model: CatalogModel): RuntimeDraft {
  const local = object(model.local)
  const sampling = object(model.sampling)
  const context = typeof local.context_length === 'number' ? local.context_length : -1
  return {
    contextMode: context > 0 ? 'manual' : 'auto',
    contextLength: context > 0 ? String(context) : '',
    temperature: text(sampling.temperature),
    topP: text(sampling.top_p),
    topK: text(sampling.top_k),
    minP: text(sampling.min_p),
    maxTokens: text(sampling.max_tokens),
    reasoningEffort: typeof sampling.reasoning_effort === 'string' ? sampling.reasoning_effort : '',
  }
}

function number(value: string): number | null {
  const trimmed = value.trim()
  if (!trimmed) return null
  const parsed = Number(trimmed)
  return Number.isFinite(parsed) ? parsed : Number.NaN
}

export function validateRuntime(
  draft: RuntimeDraft,
  limit: number | null,
): Partial<Record<Field | 'reasoningEffort', string>> {
  const errors: Partial<Record<Field | 'reasoningEffort', string>> = {}
  const check = (field: Field, ok: (n: number) => boolean, message: string) => {
    const parsed = number(draft[field])
    if (parsed !== null && (Number.isNaN(parsed) || !ok(parsed))) errors[field] = message
  }
  if (draft.contextMode === 'manual') {
    const parsed = number(draft.contextLength)
    if (parsed === null || Number.isNaN(parsed) || !Number.isInteger(parsed))
      errors.contextLength = 'Enter a whole number of tokens.'
    else if (parsed < MIN_CONTEXT) errors.contextLength = `Use at least ${MIN_CONTEXT} tokens.`
    else if (limit !== null && parsed > limit)
      errors.contextLength = `This model supports at most ${limit} tokens.`
  }
  check('temperature', (n) => n >= 0 && n <= 5, 'Use 0 to 5.')
  check('topP', (n) => n > 0 && n <= 1, 'Use a number above 0 and up to 1.')
  check('topK', (n) => Number.isInteger(n) && n >= 0, 'Use a whole number, 0 or more.')
  check('minP', (n) => n >= 0 && n <= 1, 'Use 0 to 1.')
  check('maxTokens', (n) => Number.isInteger(n) && n >= 1, 'Use a whole number, 1 or more.')
  if (
    draft.reasoningEffort &&
    !(REASONING_EFFORTS as readonly string[]).includes(draft.reasoningEffort)
  )
    errors.reasoningEffort = 'Choose none, low, medium or high.'
  return errors
}

export function applyRuntime(catalog: Catalog, id: string, draft: RuntimeDraft): Catalog {
  const models = catalog.models.map((model) => {
    if (model.id !== id) return model
    const next: CatalogModel = { ...model }
    if (isLocal(model)) {
      next.local = {
        ...object(model.local),
        context_length: draft.contextMode === 'manual' ? Number(draft.contextLength) : -1,
      }
    }
    const sampling = { ...object(model.sampling) }
    for (const { field, key } of SAMPLING) {
      const parsed = number(draft[field])
      if (parsed === null) delete sampling[key]
      else sampling[key] = parsed
    }
    if (draft.reasoningEffort) sampling.reasoning_effort = draft.reasoningEffort
    else delete sampling.reasoning_effort
    if (Object.keys(sampling).length) next.sampling = sampling
    else delete next.sampling
    return next
  })
  return { ...catalog, models }
}

export const sameRuntime = (a: RuntimeDraft, b: RuntimeDraft) =>
  JSON.stringify(a) === JSON.stringify(b)