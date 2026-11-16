export type ResultValue =
  | null
  | boolean
  | number
  | string
  | ResultValue[]
  | { [key: string]: ResultValue }

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