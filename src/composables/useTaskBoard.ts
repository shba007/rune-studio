import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { friendly } from '@/config/errors'

export type TaskKind = 'background' | 'desktop'
export type TaskPhase =
  | 'queued'
  | 'planning'
  | 'running'
  | 'done'
  | 'failed'
  | 'cancelled'
  | 'blocked'
export interface PlannedRequest {
  startedAt?: number | null
  finishedAt?: number | null
  dependsOn: number[]
  id: number
  text: string
  phase: TaskPhase
  error: string | null
  goals: { id: string; title: string; dependsOn: string[]; tasks: number[] }[]
}
export interface IndependentTask {
  currentStep?: number
  startedAt?: number | null
  finishedAt?: number | null
  stepStartedAt?: number | null
  id: number
  text: string
  kind: TaskKind
  requestId: number
  goalId: string
  dependsOn: number[]
  steps: { text: string; phase: TaskPhase }[]
  phase: TaskPhase
  result: string | null
}
interface Snapshot {
  revision: number
  tasks: IndependentTask[]
  requests: PlannedRequest[]
}
export function useTaskBoard() {
  const tasks = ref<IndependentTask[]>([])
  const requests = ref<PlannedRequest[]>([])
  const notice = ref('')
  const pending = ref(false)
  let off: UnlistenFn | undefined
  let disposed = false
  let revision = -1
  function update(snapshot: Snapshot) {
    if (!disposed && snapshot && Array.isArray(snapshot.tasks) && snapshot.revision >= revision) {
      revision = snapshot.revision
      tasks.value = snapshot.tasks
      requests.value = Array.isArray(snapshot.requests) ? snapshot.requests : []
    }
  }
  onMounted(async () => {
    try {
      const unlisten = await listen<Snapshot>('tasks_updated', (event) => update(event.payload))
      if (disposed) {
        unlisten()
        return
      }
      off = unlisten
      update(await invoke<Snapshot>('tasks_state'))
    } catch (error) {
      notice.value = friendly(String(error))
    }
  })
  onBeforeUnmount(() => {
    disposed = true
    off?.()
  })
  async function submit(text: string) {
    if (pending.value) return false
    pending.value = true
    notice.value = ''
    try {
      await invoke('submit_independent_task', { text })
      return true
    } catch (error) {
      notice.value = friendly(String(error))
      return false
    } finally {
      pending.value = false
    }
  }
  async function action(command: string, id: number) {
    try {
      await invoke(command, { id })
    } catch (error) {
      notice.value = friendly(String(error))
    }
  }
  return {
    tasks,
    requests,
    notice,
    pending,
    submit,
    cancel: (id: number) => action('cancel_independent_task', id),
    cancelRequest: (id: number) => action('cancel_planned_request', id),
    dismiss: (id: number) => action('dismiss_planned_request', id),
  }
}
export type TaskBoardModel = ReturnType<typeof useTaskBoard>