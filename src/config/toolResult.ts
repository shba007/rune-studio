export type ResultValue =
  | null
  | boolean
  | number
  | string
  | ResultValue[]
  | { [key: string]: ResultValue }

export interface SchemaField {
  key: string
  label: string
  hint?: string
}
export type SchemaView =
  | { kind: 'json'; value: ResultValue }
  | { kind: 'value'; value: ResultValue }
  | { kind: 'missing' }
  | { kind: 'object'; fields: (SchemaField & { node: SchemaView })[] }
  | { kind: 'list'; items: SchemaView[] }
  | { kind: 'table'; columns: SchemaField[]; rows: SchemaView[][] }

const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value)
const field = (key: string, schema: unknown): SchemaField => ({
  key,
  label: record(schema) && typeof schema.title === 'string' ? schema.title : resultLabel(key),
  hint: record(schema) && typeof schema.description === 'string' ? schema.description : undefined,
})

export function schemaView(schema: unknown, value: ResultValue, depth = 0): SchemaView {
  const fallback: SchemaView = { kind: 'json', value }
  if (
    depth >= 12 ||
    !record(schema) ||
    ['$ref', 'anyOf', 'oneOf', 'allOf', 'not', 'if', 'then', 'else'].some((key) => key in schema)
  )
    return fallback
  if (schema.type === 'object' && record(value)) {
    const properties = record(schema.properties) ? schema.properties : {}
    const keys = [
      ...Object.keys(properties),
      ...Object.keys(value).filter((key) => !Object.hasOwn(properties, key)),
    ]
    return {
      kind: 'object',
      fields: keys.map((key) => ({
        ...field(key, properties[key]),
        node: Object.hasOwn(value, key)
          ? schemaView(properties[key], value[key] as ResultValue, depth + 1)
          : { kind: 'missing' },
      })),
    }
  }
  if (schema.type === 'array' && Array.isArray(value) && record(schema.items)) {
    const items = schema.items
    if (
      items.type === 'object' &&
      record(items.properties) &&
      value.every(record) &&
      !['$ref', 'anyOf', 'oneOf', 'allOf', 'not', 'if', 'then', 'else'].some((key) => key in items)
    ) {
      const properties = items.properties
      const rows = value as Record<string, ResultValue>[]
      const keys = [
        ...Object.keys(properties),
        ...new Set(
          rows.flatMap((row) => Object.keys(row)).filter((key) => !Object.hasOwn(properties, key)),
        ),
      ]
      return {
        kind: 'table',
        columns: keys.map((key) => field(key, properties[key])),
        rows: rows.map((row) =>
          keys.map((key) =>
            Object.hasOwn(row, key)
              ? schemaView(properties[key], row[key]!, depth + 1)
              : { kind: 'missing' },
          ),
        ),
      }
    }
    if (['string', 'number', 'integer', 'boolean', 'null'].includes(String(items.type)))
      return { kind: 'list', items: value.map((item) => schemaView(items, item, depth + 1)) }
    return fallback
  }
  if (
    (schema.type === 'null' && value === null) ||
    (schema.type === 'integer' && typeof value === 'number' && Number.isInteger(value)) ||
    (['string', 'number', 'boolean'].includes(String(schema.type)) && typeof value === schema.type)
  )
    return { kind: 'value', value }
  return fallback
}

export function parseResult(text: string): ResultValue {
  try {
    return JSON.parse(text) as ResultValue
  } catch {
    return text
  }
}

export function resultContent(text: string): ResultValue[] {
  const value = parseResult(text)
  if (
    value &&
    !Array.isArray(value) &&
    typeof value === 'object' &&
    Array.isArray(value.content) &&
    value.content.every(
      (block) =>
        block &&
        !Array.isArray(block) &&
        typeof block === 'object' &&
        typeof block.type === 'string',
    )
  ) {
    return value.content.length
      ? value.content.map((block) => {
          const item = block as { [key: string]: ResultValue }
          return item.type === 'text' && typeof item.text === 'string'
            ? parseResult(item.text)
            : item
        })
      : [value]
  }
  return [value]
}

export function resultLabel(key: string): string {
  const words = key.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/[_-]+/g, ' ')
  return words.charAt(0).toUpperCase() + words.slice(1)
}