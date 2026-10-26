import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { PersonalityBadge, PersonalityCatalog } from '@/types/personalities'
export function usePersonality() {
  const active = ref<PersonalityBadge>({ id: 'base', name: 'Rune', mark: 'companion' })
  let off: (() => void) | undefined
  let disposed = false
  let revision = 0
  onMounted(async () => {
    try {
      const unlisten = await listen<PersonalityBadge>('personality_active', ({ payload }) => {
        revision++
        active.value = payload
      })
      if (disposed) {
        unlisten()
        return
      }
      off = unlisten
      const before = revision
      const catalog = await invoke<PersonalityCatalog>('personalities_state')
      if (!disposed && revision === before && catalog?.active) active.value = catalog.active
    } catch {
      /* The base mark remains available when initialization fails. */
    }
  })
  onBeforeUnmount(() => {
    disposed = true
    off?.()
  })
  return active
}