import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { EMPTY_SNAPSHOT, readSnapshot } from '@/config/analysis'
import { ANALYSIS_UPDATED, type AnalysisSnapshot } from '@/types/protocol'

/** The analysis window's state: the backend's snapshot, kept current by `analysis_updated`. */
export function useAnalysis() {
  const snapshot = ref<AnalysisSnapshot>({ ...EMPTY_SNAPSHOT })
  /** A refusal ("a task is running") or a failed command: shown until the next try. */
  const notice = ref<string | null>(null)
  let unlisten: UnlistenFn | undefined

  const busy = computed(
    () => snapshot.value.status === 'capturing' || snapshot.value.status === 'asking',
  )

  async function run(command: string, args?: Record<string, unknown>) {
    notice.value = null
    try {
      await invoke(command, args)
    } catch (err) {
      notice.value = String(err)
    }
  }

  /** A fresh capture; the question is optional. */
  const capture = (question?: string) => run('analyze_screen', { question: question || null })
  /** The same screen, a new question. */
  const ask = (question: string) => run('analyze_again', { question })
  const cancel = () => run('cancel_analysis')

  onMounted(async () => {
    unlisten = await listen(ANALYSIS_UPDATED, (event) => {
      snapshot.value = readSnapshot(event.payload)
    })
    // A window that opens after the analysis started catches up here.
    try {
      snapshot.value = readSnapshot(await invoke('analysis_state'))
    } catch (err) {
      notice.value = String(err)
    }
  })
  onBeforeUnmount(() => unlisten?.())

  return { snapshot, notice, busy, capture, ask, cancel }
}