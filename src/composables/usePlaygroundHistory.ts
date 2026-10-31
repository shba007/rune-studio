import { computed, ref } from 'vue'

import type { AnalysisSnapshot } from '@/types/protocol'

export interface PlaygroundRun {
  id: string
  pipeline: string
  snapshot: AnalysisSnapshot
}

const runs = ref<PlaygroundRun[]>([])
const viewedId = ref<string | null>(null)

export function usePlaygroundHistory() {
  const viewed = computed(() => runs.value.find((run) => run.id === viewedId.value) ?? null)

  function record(pipeline: string, snapshot: AnalysisSnapshot) {
    if (snapshot.status !== 'done' || !snapshot.id) return
    const run = { id: snapshot.id, pipeline, snapshot }
    const index = runs.value.findIndex((item) => item.id === run.id)
    if (index === -1) runs.value = [run, ...runs.value]
    else runs.value = runs.value.map((item, at) => (at === index ? run : item))
  }

  const view = (id: string | null) => (viewedId.value = id)
  const clearView = () => (viewedId.value = null)

  return { runs, viewed, record, view, clearView }
}