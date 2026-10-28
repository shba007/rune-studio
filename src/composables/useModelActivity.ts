import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

export interface ModelActivity {
  id: number
  model: string
  reported: boolean
}

export function useModelActivity() {
  const active = ref<ModelActivity[]>([])
  const models = computed(() => [
    ...new Set(active.value.map((item) => item.model).filter(Boolean)),
  ])
  let off: (() => void) | undefined
  let disposed = false
  let revision = 0
  onMounted(async () => {
    try {
      const stop = await listen<ModelActivity[]>('model_activity', ({ payload }) => {
        revision++
        active.value = Array.isArray(payload) ? payload : []
      })
      if (disposed) {
        stop()
        return
      }
      off = stop
      const before = revision
      const snapshot = await invoke<ModelActivity[]>('model_activity')
      if (!disposed && revision === before && Array.isArray(snapshot)) active.value = snapshot
    } catch {
      /* Model identity remains unavailable. */
    }
  })
  onBeforeUnmount(() => {
    disposed = true
    off?.()
  })
  return { active, models }
}