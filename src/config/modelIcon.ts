import { invoke } from '@tauri-apps/api/core'

export const GENERIC_MODEL_ICON = '/model/model.svg'

const loaded = new Map<string, Promise<string>>()

export function loadModelIcon(model: string): Promise<string> {
  const id = model.trim()
  if (!id) return Promise.resolve(GENERIC_MODEL_ICON)
  let icon = loaded.get(id)
  if (!icon) {
    icon = invoke<string | null>('infer_model_icon', { model: id })
      .then((uri) => uri || GENERIC_MODEL_ICON)
      .catch(() => {
        loaded.delete(id)
        return GENERIC_MODEL_ICON
      })
    loaded.set(id, icon)
  }
  return icon
}

export function clearModelIcons() {
  loaded.clear()
}