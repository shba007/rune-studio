import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useIntervalFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { todaysChips, type Habit } from '@/config/habits'

const REFRESH_EVENTS = ['workspace_tasks_changed', 'workspace_tasks_due', 'focus_state_changed']

export function useHabits() {
  const habits = ref<Habit[]>([])
  const error = ref('')
  const pending = ref(false)
  const now = ref(Date.now())
  const chips = computed(() => todaysChips(habits.value, now.value))

  async function refresh() {
    try {
      const result = await invoke<Habit[]>('habits_state')
      habits.value = Array.isArray(result) ? result : []
    } catch (e) {
      error.value = String(e)
    }
  }

  // The day can change while the pill stays open, so the chips are re-read and
  // re-dated every minute.
  useIntervalFn(() => {
    now.value = Date.now()
    void refresh()
  }, 60_000)

  async function act(command: string, args: Record<string, unknown>) {
    if (pending.value) return false
    pending.value = true
    error.value = ''
    try {
      await invoke(command, args)
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      pending.value = false
      await refresh()
    }
  }

  const checkIn = (id: string) => act('habit_check_in', { id })
  const startFocus = (id: string) => act('focus_start_task', { taskId: id })

  const off: UnlistenFn[] = []
  let disposed = false
  onMounted(async () => {
    for (const name of REFRESH_EVENTS) {
      try {
        const stop = await listen(name, () => void refresh())
        if (disposed) stop()
        else off.push(stop)
      } catch (e) {
        error.value = String(e)
      }
    }
    await refresh()
  })
  onBeforeUnmount(() => {
    disposed = true
    off.splice(0).forEach((stop) => stop())
  })

  return { habits, chips, error, pending, refresh, checkIn, startFocus }
}

export type HabitsModel = ReturnType<typeof useHabits>