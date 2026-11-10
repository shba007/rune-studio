export function mcpNamespace(packageName: string): string {
  return packageName.replace(/^rune-/, '')
}

export function toolsOf(packageName: string, toolNames: string[]): string[] {
  const namespace = mcpNamespace(packageName)
  return toolNames.filter((name) => name.split('__')[0] === namespace)
}

export function isEnabled(packageName: string, toolNames: string[], selected: string[]): boolean {
  return toolsOf(packageName, toolNames).some((name) => selected.includes(name))
}

export function withEnabled(
  packageName: string,
  toolNames: string[],
  selected: string[],
  enabled: boolean,
): string[] {
  const tools = toolsOf(packageName, toolNames)
  const others = selected.filter((name) => !tools.includes(name))
  return enabled ? [...others, ...tools] : others
}