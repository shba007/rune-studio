export interface KitTool {
  name: string
  description: string
  inputSchema: unknown
}
export interface ToolField {
  name: string
  label: string
  description: string
  type: 'string' | 'number' | 'integer' | 'boolean'
  required: boolean
  options?: (string | number | boolean)[]
}
export type ToolForm = { raw: true } | { raw: false; fields: ToolField[] }
const object = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)
const complex = [
  'anyOf',
  'oneOf',
  'allOf',
  'not',
  '$ref',
  'if',
  'then',
  'else',
  'dependentSchemas',
  'dependentRequired',
  'dependencies',
  'patternProperties',
  'unevaluatedProperties',
  'const',
]

export function toolForm(schema: unknown): ToolForm {
  if (!object(schema) || schema.type !== 'object' || complex.some((key) => key in schema))
    return { raw: true }
  if (schema.properties !== undefined && !object(schema.properties)) return { raw: true }
  if (object(schema.additionalProperties)) return { raw: true }
  if (
    schema.required !== undefined &&
    (!Array.isArray(schema.required) || schema.required.some((key) => typeof key !== 'string'))
  )
    return { raw: true }
  const properties = (schema.properties ?? {}) as Record<string, unknown>
  const required = (schema.required ?? []) as string[]
  if (required.some((key) => !Object.hasOwn(properties, key))) return { raw: true }
  const fields: ToolField[] = []
  for (const [name, property] of Object.entries(properties)) {
    if (!object(property) || complex.some((key) => key in property)) return { raw: true }
    const enumType =
      Array.isArray(property.enum) && property.enum.length ? typeof property.enum[0] : undefined
    const type = property.type ?? enumType
    if (type !== 'string' && type !== 'number' && type !== 'integer' && type !== 'boolean')
      return { raw: true }
    let options: ToolField['options']
    if (property.enum !== undefined) {
      if (
        !Array.isArray(property.enum) ||
        !property.enum.length ||
        property.enum.some((value) =>
          type === 'integer' ? !Number.isInteger(value) : typeof value !== type,
        )
      )
        return { raw: true }
      options = property.enum as NonNullable<ToolField['options']>
    }
    fields.push({
      name,
      label: typeof property.title === 'string' ? property.title : name,
      description: typeof property.description === 'string' ? property.description : '',
      type,
      required: required.includes(name),
      options,
    })
  }
  return { raw: false, fields }
}

export function toolArguments(
  form: ToolForm,
  values: Record<string, string>,
  raw: string,
): Record<string, unknown> {
  if (form.raw) {
    const parsed: unknown = JSON.parse(raw)
    if (!object(parsed)) throw new Error('Arguments must be a JSON object')
    return parsed
  }
  const entries: [string, unknown][] = []
  for (const field of form.fields) {
    const value = values[field.name] ?? ''
    if (value === '') {
      if (field.required) {
        if (field.type === 'string' && !field.options) entries.push([field.name, ''])
        else throw new Error(`${field.label} is required`)
      }
      continue
    }
    let parsed: unknown = value
    if (field.options) {
      const index = Number(value)
      if (!Number.isInteger(index) || field.options[index] === undefined)
        throw new Error(`Choose ${field.label}`)
      parsed = field.options[index]
    } else if (field.type === 'boolean') {
      if (value !== 'true' && value !== 'false') throw new Error(`Choose ${field.label}`)
      parsed = value === 'true'
    } else if (field.type === 'number' || field.type === 'integer') {
      parsed = Number(value)
      if (!Number.isFinite(parsed) || (field.type === 'integer' && !Number.isInteger(parsed)))
        throw new Error(
          `${field.label} must be ${field.type === 'integer' ? 'an integer' : 'a number'}`,
        )
    }
    entries.push([field.name, parsed])
  }
  return Object.fromEntries(entries)
}

export function toolResultText(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
}