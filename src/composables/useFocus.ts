import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useIntervalFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
  DEFAULT_PRESET,
  type CalendarReport,
  EMPTY_FOCUS,
  freeMs,
  isOver,
  remainingMs,
  wrapUpFor,
  type FocusSnapshot,
  type FocusSuggestion,
} from '@/config/focus'

const formOpen = ref(false)
export const openFocusForm = () => {
  formOpen.value = true
}

export const closeFocusForm = () => {
  formOpen.value = false
}

export function useFocus() {
  const snapshot = ref<FocusSnapshot>(EMPTY_FOCUS)
  const error = ref('')
  const pending = ref(false)
  const suggesting = ref(false)
  const now = ref(Date.now())
  const block = computed(() => snapshot.value.block)
  const active = computed(() => block.value !== null)
  const remaining = computed(() => (block.value ? remainingMs(block.value, now.value) : 0))
  const commitments = computed(() => snapshot.value.commitments)
  const preset = computed(() => snapshot.value.preset ?? DEFAULT_PRESET)
  const free = computed(() => freeMs(commitments.value, now.value, preset.value))
  const wrapUp = computed(() =>
    block.value ? (wrapUpFor(commitments.value, now.value, preset.value) ?? null) : null,
  )
  const over = computed(() => (block.value ? isOver(block.value, now.value) : false))

  const ticker = useIntervalFn(() => (now.value = Date.now()), 1000, { immediate: false })
  watch(
    () => active.value || formOpen.value,
    (running) => {
      if (running) {
        now.value = Date.now()
        ticker.resume()
      } else ticker.pause()
    },
    { immediate: true },
  )

  async function call(command: string, args?: Record<string, unknown>) {
    if (pending.value) return false
    pending.value = true
    error.value = ''
    try {
      snapshot.value = await invoke<FocusSnapshot>(command, args)
      return true
    } catch (e) {
      error.value = String(e)
      return false
    } finally {
      pending.value = false
    }
  }

  async function refresh() {
    try {
      snapshot.value = await invoke<FocusSnapshot>('focus_snapshot')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function start(title: string, doneWhen: string, minutes: number) {
    const started = await call('focus_start', { title, doneWhen, minutes })
    if (started) formOpen.value = false
    return started
  }

  async function suggest(goal: string): Promise<FocusSuggestion | null> {
    if (suggesting.value) return null
    suggesting.value = true
    error.value = ''
    try {
      return await invoke<FocusSuggestion>('focus_suggest', { goal })
    } catch (e) {
      error.value = String(e)
      return null
    } finally {
      suggesting.value = false
    }
  }

  const extend = () => call('focus_extend')
  const handOff = () => call('focus_hand_off')
  const addCommitment = (title: string, startMs: number, minutes: number) =>
    call('focus_commitment_add', { title, startMs, minutes })
  const removeCommitment = (id: number) => call('focus_commitment_remove', { id })

  const calendar = ref<CalendarReport | null>(null)
  async function syncCalendar() {
    if (pending.value) return
    pending.value = true
    try {
      calendar.value = await invoke<CalendarReport>('focus_calendar_sync')
    } catch (e) {
      calendar.value = { state: 'unavailable', events: 0, message: String(e) }
    } finally {
      pending.value = false
    }
    await refresh()
  }

  async function finish(outcome: 'done' | 'stopped') {
    const finished = await call('focus_finish', { outcome })
    if (finished) formOpen.value = true
    return finished
  }

  const hotkeyTick = ref(0)
  const dictating = ref(false)
  const dictated = ref<{ text: string; n: number } | null>(null)
  async function setDictation(on: boolean) {
    if (dictating.value === on) return true
    error.value = ''
    try {
      await invoke('focus_dictation_set', { on })
      dictating.value = on
      return true
    } catch (e) {
      error.value = String(e)
      return false
    }
  }

  const off: UnlistenFn[] = []
  let disposed = false
  async function on<T>(name: string, run: (payload: T) => void) {
    try {
      const stop = await listen<T>(name, (event) => run(event.payload))
      if (disposed) stop()
      else off.push(stop)
    } catch (e) {
      error.value = String(e)
    }
  }
  onMounted(async () => {
    await on<FocusSnapshot>('focus_state_changed', (payload) => (snapshot.value = payload))
    await on<unknown>('focus_hotkey', () => {
      formOpen.value = true
      hotkeyTick.value++
    })
    await on<string>('focus_dictated', (text) => {
      dictated.value = { text, n: (dictated.value?.n ?? 0) + 1 }
    })
    await refresh()
  })
  onBeforeUnmount(() => {
    disposed = true
    off.splice(0).forEach((stop) => stop())
  })

  return {
    snapshot,
    block,
    active,
    remaining,
    over,
    commitments,
    preset,
    free,
    wrapUp,
    extend,
    handOff,
    addCommitment,
    removeCommitment,
    hotkeyTick,
    dictating,
    dictated,
    setDictation,
    calendar,
    syncCalendar,
    now,
    error,
    pending,
    suggesting,
    suggest,
    formOpen,
    start,
    finish,
    closeForm: closeFocusForm,
    refresh,
  }
}

export type Focus = ReturnType<typeof useFocus>