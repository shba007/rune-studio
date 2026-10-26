import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useIntervalFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { WorkspaceTask } from '@/types/workspace'
export function useWorkspaceTasks() {
  const tasks = ref<WorkspaceTask[]>([])
  const pending = ref(false)
  const error = ref('')
  const loaded = ref(false)
  let sequence = 0
  let disposed = false
  const off: (() => void)[] = []
  async function refresh() {
    const token = ++sequence
    try {
      const result = await invoke<WorkspaceTask[]>('workspace_tasks_state')
      if (!disposed && token === sequence) {
        tasks.value = Array.isArray(result) ? result : []
        loaded.value = true
      }
    } catch (e) {
      if (!disposed && token === sequence) error.value = String(e)
    }
  }
  async function call<T>(command: string, args: Record<string, unknown>): Promise<T | null> {
    if (pending.value) return null
    pending.value = true
    error.value = ''
    try {
      return await invoke<T>(command, args)
    } catch (e) {
      error.value = String(e)
      return null
    } finally {
      pending.value = false
      await refresh()
    }
  }
  onMounted(async () => {
    for (const name of ['workspace_tasks_changed', 'workspace_tasks_due', 'settings_changed']) {
      try {
        const stop = await listen(name, () => void refresh())
        if (disposed) {
          stop()
          return
        }
        off.push(stop)
      } catch (e) {
        error.value = String(e)
      }
    }
    await refresh()
  })
  useIntervalFn(() => {
    if (!disposed) void refresh()
  }, 5000)
  onBeforeUnmount(() => {
    disposed = true
    off.forEach((stop) => stop())
  })
  const due = computed(() =>
    tasks.value.filter(
      (t) =>
        !t.paused &&
        ['pending', 'in_progress'].includes(t.occurrences.at(-1)?.state ?? '') &&
        t.occurrences.at(-1)?.dueMs != null &&
        t.occurrences.at(-1)!.dueMs! <= Date.now(),
    ),
  )
  return {
    tasks,
    pending,
    error,
    loaded,
    due,
    refresh,
    call,
    action: (task: WorkspaceTask, action: string) =>
      call('workspace_task_action', { id: task.id, revision: task.revision, action }),
    link: (task: WorkspaceTask, parentGoalId: string | null) =>
      call('workspace_task_link', { id: task.id, revision: task.revision, parentGoalId }),
  }
}
export type WorkspaceTasksModel = ReturnType<typeof useWorkspaceTasks>