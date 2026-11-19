// Helpers for the one-click Apply card shown where a write was staged by Kit.

// "kit__rune-filesystem__write_file" and "rune-filesystem__write_file" both belong to the
// plugin "rune-filesystem".
export function pluginOf(tool: string): string {
  const name = tool.startsWith('kit__') ? tool.slice('kit__'.length) : tool
  return name.split('__')[0] ?? name
}

// A write tool reports `staged: true` in its structured result when Kit kept it for review.
export function wasStaged(structuredContent: unknown): boolean {
  return (
    !!structuredContent &&
    typeof structuredContent === 'object' &&
    (structuredContent as Record<string, unknown>).staged === true
  )
}

export function describeChange(review: { deleted: boolean; baseline: string | null }): string {
  if (review.deleted) return 'Deletes this file'
  return review.baseline ? 'Replaces this file' : 'Creates this file'
}
// One entry per plugin that staged something in these steps. `count` changes when a later step
// stages again, so a card showing that plugin reloads and lists the new change.
export function stagedPlugins(
  steps: { kind: string; tool: string; result?: { structuredContent?: unknown } | null }[],
): { plugin: string; tool: string; count: number }[] {
  const found = new Map<string, { plugin: string; tool: string; count: number }>()
  for (const step of steps) {
    if (step.kind !== 'tool' || !wasStaged(step.result?.structuredContent)) continue
    const plugin = pluginOf(step.tool)
    const entry = found.get(plugin)
    if (entry) entry.count += 1
    else found.set(plugin, { plugin, tool: step.tool, count: 1 })
  }
  return [...found.values()]
}