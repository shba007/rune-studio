import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import type { MindsetCatalog } from '@/types/mindsets'

export function useMindsetCatalog() {
  const entries = ref<{ id: string; name: string }[]>([])
  async function refresh() {
    try {
      const catalog = await invoke<MindsetCatalog>('personalities_state')
      entries.value = catalog.entries.map(({ id, name }) => ({ id, name }))
    } catch {
      entries.value = []
    }
  }
  return { entries, refresh }
}