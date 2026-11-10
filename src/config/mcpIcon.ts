import { invoke } from '@tauri-apps/api/core'

const loaded = new Map<string, Promise<string | null>>()

export function loadMcpIcon(name: string): Promise<string | null> {
  const id = name.trim()
  if (!id) return Promise.resolve(null)
  let icon = loaded.get(id)
  if (!icon) {
    icon = invoke<unknown>('kit_settings')
      .then((config) => invoke<string | null>('kit_icon', { config, name: id }))
      .then((uri) => uri || null)
      .catch(() => {
        loaded.delete(id)
        return null
      })
    loaded.set(id, icon)
  }
  return icon
}

export function clearMcpIcons() {
  loaded.clear()
}