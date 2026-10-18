import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTimeoutFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
  AUDIO_LEVEL,
  OPEN_SETTINGS,
  STATE_CHANGED,
  SUMMON,
  type AgentState,
  type BackendStatus,
  type PillMode,
  type Settings,
  type StateChangedPayload,
} from '@/types/protocol'

const RUNNING: AgentState[] = ['planning', 'awaiting_confirm', 'acting', 'observing']

const STATE_LABELS: Record<AgentState, string> = {
  idle: 'Ready',
  listening: 'Listening',
  planning: 'Thinking',
  awaiting_confirm: 'Needs your OK',
  acting: 'Acting',
  observing: 'Looking',
  done: 'Done',
  failed: 'Failed',
  stopped: 'Stopped',
}

/** Window size order. Bigger modes need a bigger window than smaller ones. */
const MODE_RANK: Record<PillMode, number> = { ambient: 0, open: 1, settings: 2 }

/** Longer than the page and width transitions, so the window only shrinks once they have finished. */
const COLLAPSE_MS = 340
const NOTICE_MS = 4000
/** How long the mic may stay silent before we suggest it is not getting any audio. */
const SILENCE_MS = 4000
const SOUND_THRESHOLD = 0.04

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

/** Thin view-model over the Rust side: every piece of state here arrives as an event or command result. */
export function useAgent() {
  const state = ref<AgentState>('idle')
  const backend = ref<BackendStatus | null>(null)
  const settings = ref<Settings | null>(null)
  const hasApiKey = ref(false)
  const mode = ref<PillMode>('ambient')
  const notice = ref<string | null>(null)
  const focusTick = ref(0)
  /** Live microphone level, 0..1. Stays 0 unless the mic is open. */
  const level = ref(0)

  const isRunning = computed(() => RUNNING.includes(state.value))
  const statusText = computed(() => notice.value ?? STATE_LABELS[state.value])

  const unlisten: UnlistenFn[] = []
  let modeToken = 0
  let heardSound = false

  // `start` restarts a running timer, so a newer notice always gets its full time on screen.
  const noticeTimer = useTimeoutFn(() => (notice.value = null), NOTICE_MS, { immediate: false })
  const silenceTimer = useTimeoutFn(
    () => {
      if (!heardSound) showNotice('No sound from the microphone. Check Windows microphone access')
    },
    SILENCE_MS,
    { immediate: false },
  )

  function showNotice(message: string) {
    notice.value = message
    noticeTimer.start()
  }

  async function call<T>(command: string, args?: Record<string, unknown>): Promise<T | undefined> {
    try {
      return await invoke<T>(command, args)
    } catch (err) {
      showNotice(String(err))
      return undefined
    }
  }

  /**
   * A transparent window still catches clicks, and clips whatever does not fit. So when the
   * target needs a bigger window (ambient < open < settings) the window grows first and the
   * page animates in afterwards; when it needs a smaller one the page animates out first and
   * the window shrinks last. This holds for every pair of modes, so no page is ever drawn
   * into a window that is too small. A newer call cancels the tail of an older one.
   */
  async function setMode(next: PillMode) {
    const token = ++modeToken
    const grows = MODE_RANK[next] > MODE_RANK[mode.value]
    if (grows) {
      await call('set_mode', { mode: next })
      if (token !== modeToken) return
      mode.value = next
    } else {
      mode.value = next
      await sleep(COLLAPSE_MS)
      if (token === modeToken) await call('set_mode', { mode: next })
    }
    if (next === 'open') focusTick.value += 1
  }

  /** Open when collapsed; collapse when open or in settings. */
  const toggleOpen = () => setMode(mode.value === 'ambient' ? 'open' : 'ambient')
  const dismiss = () => (mode.value === 'ambient' ? Promise.resolve() : setMode('ambient'))

  async function refreshBackend() {
    backend.value = (await call<BackendStatus>('check_backend')) ?? null
  }

  async function refreshSettings() {
    settings.value = (await call<Settings>('get_settings')) ?? null
    hasApiKey.value = (await call<boolean>('has_api_key')) ?? false
  }

  async function saveSettings(next: Settings) {
    await call('save_settings', { settings: next })
    await refreshSettings()
    await refreshBackend()
  }

  async function saveApiKey(key: string) {
    await call('set_api_key', { key })
    await refreshSettings()
    await refreshBackend()
  }

  /** Resolves to whether the task was accepted, so the composer only clears on success. */
  async function startTask(text: string): Promise<boolean> {
    try {
      await invoke('start_task', { text })
      return true
    } catch (err) {
      showNotice(String(err))
      return false
    }
  }

  const stop = () => call('stop')
  const toggleListening = () => call('toggle_listening')
  const confirm = (id: string, approved: boolean) => call('confirm', { id, approved })
  const debugSetState = (next: AgentState) => call('debug_set_state', { state: next })

  // The agent needs the user: open the card by itself.
  watch(state, (next) => {
    if (next === 'awaiting_confirm' && mode.value === 'ambient') void setMode('open')
  })

  // Tell the user when "listening" is not actually hearing anything.
  watch(state, (next) => {
    silenceTimer.stop()
    heardSound = false
    if (next !== 'listening') {
      level.value = 0
      return
    }
    silenceTimer.start()
  })

  onMounted(async () => {
    unlisten.push(
      await listen<StateChangedPayload>(STATE_CHANGED, (event) => {
        state.value = event.payload.state
      }),
      await listen<number>(AUDIO_LEVEL, (event) => {
        level.value = event.payload
        if (event.payload > SOUND_THRESHOLD) heardSound = true
      }),
      await listen(SUMMON, () => void toggleOpen()),
      await listen(OPEN_SETTINGS, () => void setMode('settings')),
    )
    await refreshSettings()
    await refreshBackend()
  })

  onBeforeUnmount(() => unlisten.forEach((off) => off()))

  return {
    state,
    isRunning,
    statusText,
    backend,
    settings,
    hasApiKey,
    mode,
    notice,
    focusTick,
    level,
    setMode,
    toggleOpen,
    dismiss,
    refreshBackend,
    saveSettings,
    saveApiKey,
    startTask,
    stop,
    toggleListening,
    confirm,
    debugSetState,
  }
}

export type Agent = ReturnType<typeof useAgent>