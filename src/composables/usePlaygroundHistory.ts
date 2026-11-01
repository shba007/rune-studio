import { computed, ref } from 'vue'

import type { AnalysisSnapshot } from '@/types/protocol'
import type {
  CaptionResult,
  CreateResult,
  DepthResult,
  EditResult,
  SpeechResult,
  UpscaleResult,
} from '@/config/playground'

export type ModelHistoryResult =
  | CaptionResult
  | CreateResult
  | DepthResult
  | EditResult
  | SpeechResult
  | UpscaleResult
  | { text: string; model: string; elapsedMs: number }
export type PlaygroundRun =
  | { kind: 'insight'; id: string; pipeline: string; snapshot: AnalysisSnapshot; details: string[] }
  | { kind: 'model'; id: string; pipeline: string; result: ModelHistoryResult; details: string[] }

export const MAX_HISTORY_RUNS = 10
export const MAX_HISTORY_BYTES = 50 * 1024 * 1024
const runs = ref<PlaygroundRun[]>([])
const viewedId = ref<string | null>(null)
const notice = ref('')
let nextId = 0

function memoryBytes(value: unknown): number {
  if (typeof value === 'string') return value.length * 2 + 32
  if (Array.isArray(value)) return value.reduce((sum, item) => sum + 8 + memoryBytes(item), 32)
  if (value && typeof value === 'object')
    return Object.entries(value).reduce(
      (sum, [key, item]) => sum + key.length * 2 + 32 + memoryBytes(item),
      64,
    )
  return 8
}

export function usePlaygroundHistory() {
  const viewed = computed(() => runs.value.find((run) => run.id === viewedId.value) ?? null)

  function add(run: PlaygroundRun) {
    const size = memoryBytes(run)
    if (size > MAX_HISTORY_BYTES) {
      notice.value = 'This result is too large to save in history (50 MiB limit).'
      return
    }
    const previous = runs.value.filter((item) => item.id !== run.id)
    const kept = [run, ...previous]
    let used = size
    let end = 1
    while (end < kept.length && end < MAX_HISTORY_RUNS) {
      const next = memoryBytes(kept[end])
      if (used + next > MAX_HISTORY_BYTES) break
      used += next
      end++
    }
    const evicted = kept.length - end
    runs.value = kept.slice(0, end)
    if (viewedId.value && !runs.value.some((item) => item.id === viewedId.value))
      viewedId.value = null
    notice.value = evicted
      ? `${evicted} older ${evicted === 1 ? 'run was' : 'runs were'} removed from history to stay within its limits.`
      : ''
  }

  function record(pipeline: string, snapshot: AnalysisSnapshot, details: string[] = []) {
    if (snapshot.status !== 'done' || !snapshot.id) return
    add({ kind: 'insight', id: `insight:${snapshot.id}`, pipeline, snapshot, details })
  }

  function recordModel(pipeline: string, result: ModelHistoryResult, details: string[]) {
    add({ kind: 'model', id: `model:${++nextId}`, pipeline, result, details })
  }

  const view = (id: string | null) => (viewedId.value = id)
  const clearView = () => (viewedId.value = null)
  const clear = () => {
    runs.value = []
    viewedId.value = null
    notice.value = ''
  }
  return { runs, viewed, notice, record, recordModel, view, clearView, clear }
}