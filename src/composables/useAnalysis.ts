import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { EMPTY_SNAPSHOT, readSnapshot } from '@/config/analysis'
import { friendly } from '@/config/errors'
import { ANALYSIS_UPDATED, type AnalysisSnapshot } from '@/types/protocol'

export function useAnalysis() {
  const snapshot = ref<AnalysisSnapshot>({ ...EMPTY_SNAPSHOT })
  const notice = ref<string | null>(null)
  let unlisten: UnlistenFn | undefined

  const busy = computed(
    () => snapshot.value.status === 'capturing' || snapshot.value.status === 'asking',
  )

  async function run(command: string, args?: Record<string, unknown>) {
    notice.value = null
    try {
      await invoke(command, args)
      return true
    } catch (err) {
      notice.value = friendly(String(err))
      return false
    }
  }

  const capture = (question?: string, target?: { kind: string; id: number }) =>
    target
      ? run('run_workflow', {
          workflowId: 'screen-insight',
          input: { target, question: question || '' },
        })
      : run('analyze_screen', { question: question || null })
  const ask = (question: string) => run('analyze_again', { question })
  const cancel = () => run('cancel_analysis')
  const close = () => run('close_analysis')
  const select = (n: number | null) => run('select_finding', { n })

  onMounted(async () => {
    unlisten = await listen(ANALYSIS_UPDATED, (event) => {
      snapshot.value = readSnapshot(event.payload)
    })
    try {
      snapshot.value = readSnapshot(await invoke('analysis_state'))
    } catch (err) {
      notice.value = friendly(String(err))
    }
  })
  onBeforeUnmount(() => unlisten?.())

  return { snapshot, notice, busy, capture, ask, cancel, close, select }
}