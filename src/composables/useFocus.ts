import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useIntervalFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
  EMPTY_FOCUS,
  freeMs,
  isOver,
  remainingMs,
  wrapUpFor,
  type FocusSnapshot,
  type FocusSuggestion,
} from '@/config/focus'

// The Focus card is opened from the composer (`/focus`) and from the pill, so
// whether the start form is showing is shared rather than owned by one view.
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
  const free = computed(() => freeMs(commitments.value, now.value))
  const wrapUp = computed(() =>
    block.value ? (wrapUpFor(commitments.value, now.value) ?? null) : null,
  )
  const over = computed(() => (block.value ? isOver(block.value, now.value) : false))

  // The countdown is computed from the block's end time, so it stays right
  // after a restart and needs no event per second.
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
  const addCommitment = (title: string, startMs: number, minutes: number) =>
    call('focus_commitment_add', { title, startMs, minutes })
  const removeCommitment = (id: number) => call('focus_commitment_remove', { id })

  // Finishing keeps the card open on the start form, whose list of recent
  // blocks confirms what was just logged.
  async function finish(outcome: 'done' | 'stopped') {
    const finished = await call('focus_finish', { outcome })
    if (finished) formOpen.value = true
    return finished
  }

  const off: UnlistenFn[] = []
  let disposed = false
  onMounted(async () => {
    try {
      const stop = await listen<FocusSnapshot>('focus_state_changed', (event) => {
        snapshot.value = event.payload
      })
      if (disposed) stop()
      else off.push(stop)
    } catch (e) {
      error.value = String(e)
    }
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
    free,
    wrapUp,
    extend,
    addCommitment,
    removeCommitment,
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