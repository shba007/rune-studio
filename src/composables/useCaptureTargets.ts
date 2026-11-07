import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { friendly } from '@/config/errors'

export interface CaptureTargetOption {
  target: { kind: string; id: number }
  label: string
  primary?: boolean
}

export function useCaptureTargets(emptyMessage: string) {
  const targets = ref<CaptureTargetOption[]>([])
  const refreshing = ref(false)
  const notice = ref('')

  async function load(): Promise<'loaded' | 'failed' | 'busy'> {
    if (refreshing.value) return 'busy'
    refreshing.value = true
    notice.value = ''
    try {
      const found = await invoke<unknown>('visual_analysis_targets')
      targets.value = Array.isArray(found) ? (found as CaptureTargetOption[]) : []
      if (!targets.value.length) notice.value = emptyMessage
      return 'loaded'
    } catch (error) {
      targets.value = []
      notice.value = friendly(String(error))
      return 'failed'
    } finally {
      refreshing.value = false
    }
  }

  return { targets, refreshing, notice, load }
}