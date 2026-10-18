import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import {
  FOCUS_INPUT,
  STATE_CHANGED,
  type AgentState,
  type BackendStatus,
  type Settings,
  type StateChangedPayload,
} from '@/types/protocol'

const STATE_LABELS: Record<AgentState, string> = {
  idle: 'Ready',
  listening: 'Listening…',
  planning: 'Thinking…',
  awaiting_confirm: 'Waiting for your OK',
  acting: 'Acting…',
  observing: 'Looking at the screen…',
  done: 'Done',
  failed: 'Failed',
  stopped: 'Stopped',
}

const RUNNING: AgentState[] = ['planning', 'awaiting_confirm', 'acting', 'observing']

/** Thin view-model over the Rust side: every piece of state here arrives as an event or command result. */
export function useAgent() {
  const state = ref<AgentState>('idle')
  const backend = ref<BackendStatus | null>(null)
  const settings = ref<Settings | null>(null)
  const hasApiKey = ref(false)
  const expanded = ref(false)
  const notice = ref<string | null>(null)
  const focusTick = ref(0)

  const stateLabel = computed(() => STATE_LABELS[state.value])
  const isRunning = computed(() => RUNNING.includes(state.value))

  const unlisten: UnlistenFn[] = []

  async function call<T>(command: string, args?: Record<string, unknown>): Promise<T | undefined> {
    try {
      return await invoke<T>(command, args)
    } catch (err) {
      notice.value = String(err)
      return undefined
    }
  }

  async function refreshBackend() {
    backend.value = (await call<BackendStatus>('check_backend')) ?? null
  }

  async function refreshSettings() {
    settings.value = (await call<Settings>('get_settings')) ?? null
    hasApiKey.value = (await call<boolean>('has_api_key')) ?? false
  }

  async function setExpanded(value: boolean) {
    expanded.value = value
    await call('set_expanded', { expanded: value })
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

  async function startTask(text: string) {
    notice.value = null
    await call('start_task', { text })
  }

  const stop = () => call('stop')
  const hide = () => call('hide_pill')
  const toggleListening = () => call('toggle_listening')
  const debugSetState = (next: AgentState) => call('debug_set_state', { state: next })

  onMounted(async () => {
    unlisten.push(
      await listen<StateChangedPayload>(STATE_CHANGED, (event) => {
        state.value = event.payload.state
      }),
      await listen(FOCUS_INPUT, () => {
        focusTick.value += 1
      }),
    )
    await refreshSettings()
    await refreshBackend()
  })

  onBeforeUnmount(() => unlisten.forEach((off) => off()))

  return {
    state,
    stateLabel,
    isRunning,
    backend,
    settings,
    hasApiKey,
    expanded,
    notice,
    focusTick,
    refreshBackend,
    setExpanded,
    saveSettings,
    saveApiKey,
    startTask,
    stop,
    hide,
    toggleListening,
    debugSetState,
  }
}