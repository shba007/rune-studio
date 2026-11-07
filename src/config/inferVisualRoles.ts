import type { VisualAnalysisModels } from '@/types/protocol'

export function updateVisualRoles(raw: string, models: VisualAnalysisModels): string {
  const catalog = JSON.parse(raw) as {
    roles?: Record<string, string>
    models?: { roles?: string[] }[]
  }
  const roles = { ...catalog.roles }
  const editedRoles = new Set(Object.keys(models).map((key) => `visual_${key}`))
  for (const model of catalog.models ?? []) {
    if (Array.isArray(model.roles)) {
      model.roles = model.roles.filter((role) => !editedRoles.has(role.toLowerCase()))
    }
  }
  for (const [key, value] of Object.entries(models)) {
    const role = `visual_${key}`
    if (value?.trim()) roles[role] = value.trim()
    else delete roles[role]
  }
  catalog.roles = roles
  return JSON.stringify(catalog, null, 2)
}