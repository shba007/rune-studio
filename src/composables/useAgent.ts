import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTimeoutFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
  AUDIO_LEVEL,
  VOICE_HEARD,
  OPEN_SETTINGS,
  CONFIRM_REQUEST,
  CONFIRM_RESOLVED,
  PLAN_UPDATED,
  STATE_CHANGED,
  STEP_STARTED,
  SUMMON,
  TASK_DONE,
  THINKING,
  type AgentState,
  type BackendStatus,
  type BridgeInfo,
  type CaptureReport,
  type ClickReport,
  type ConfirmRequestPayload,
  type VoiceHeardPayload,
  type PillMode,
  type PlanUpdatedPayload,
  type Settings,
  type StateChangedPayload,
  type StepStartedPayload,
  type TaskDonePayload,
  type ThinkingPayload,
} from '@/types/protocol'

const RUNNING: AgentState[] = ['planning', 'awaiting_confirm', 'acting', 'observing']

const STATE_LABELS: Record<AgentState, string> = {
  idle: 'Ready',
  listening: 'Listening',
  planning: 'Thinking',
  awaiting_confirm: 'Needs your OK',
  acting: 'Acting',
  observing: 'Looking',
  reflecting: 'Reflecting',
  done: 'Done',
  failed: 'Failed',
  stopped: 'Stopped',
}

/** Window size order. Bigger modes need a bigger window than smaller ones. */
const MODE_RANK: Record<PillMode, number> = { ambient: 0, open: 1, task: 2, settings: 3 }

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
  /** The outside-agent door. `available` is false unless the app was built with it. */
  const bridge = ref<BridgeInfo | null>(null)
  const mode = ref<PillMode>('ambient')
  /** The task's goals and the one being worked on. Null until a plan arrives. */
  const plan = ref<PlanUpdatedPayload | null>(null)
  /** What the model is thinking right now (only with Show thinking on). */
  const thinking = ref<ThinkingPayload | null>(null)
  /** What the agent last said: its reason for the current action, or the final summary. */
  const response = ref<string | null>(null)
  /** The question waiting for the user's yes or no, if any. */
  const question = ref<ConfirmRequestPayload | null>(null)
  /** The actions taken so far, newest last. */
  const steps = ref<{ step: number; tool: string; detail: string; ok: boolean | null }[]>([])
  const notice = ref<string | null>(null)
  const focusTick = ref(0)
  /** Live microphone level, 0..1. Stays 0 unless the mic is open. */
  const level = ref(0)

  const isRunning = computed(() => RUNNING.includes(state.value))

  /** How healthy the model server is, as one colour and a sentence for the tooltip. */
  const health = computed<{ tone: 'ok' | 'warn' | 'down' | 'checking'; title: string }>(() => {
    const status = backend.value
    const model = settings.value?.model ?? ''
    if (!status) return { tone: 'checking', title: 'Checking the model server' }
    if (!status.reachable) return { tone: 'down', title: status.error ?? 'Model server offline' }
    if (!status.authOk) return { tone: 'warn', title: 'API key rejected' }
    if (status.missingModels.length > 0) {
      return { tone: 'warn', title: `Missing model: ${status.missingModels.join(', ')}` }
    }
    if (status.error) return { tone: 'warn', title: status.error }
    return { tone: 'ok', title: `Model server OK, ${status.latencyMs ?? '?'} ms, ${model}` }
  })
  /** What the agent is doing right now (its reason for the current step), while a task runs. */
  const stepText = ref<string | null>(null)
  /** How the last task ended, shown while the state is done or failed. */
  const resultText = ref<string | null>(null)
  const statusText = computed(() => {
    if (notice.value) return notice.value
    if (isRunning.value && stepText.value) return stepText.value
    if ((state.value === 'done' || state.value === 'failed') && resultText.value)
      return resultText.value
    return STATE_LABELS[state.value]
  })

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
  /** Make the window as tall as the open card's content. Only the open and task pages do this. */
  async function fitHeight(height: number) {
    if (mode.value === 'open' || mode.value === 'task') await call('fit_height', { height })
  }

  /** The page the card opens to: the task's progress while there is one, else the composer. */
  const home = (): PillMode => (plan.value ? 'task' : 'open')
  const toggleOpen = () => setMode(mode.value === 'ambient' ? home() : 'ambient')
  const dismiss = () => (mode.value === 'ambient' ? Promise.resolve() : setMode('ambient'))

  async function refreshBackend() {
    backend.value = (await call<BackendStatus>('check_backend')) ?? null
  }

  async function refreshSettings() {
    settings.value = (await call<Settings>('get_settings')) ?? null
    hasApiKey.value = (await call<boolean>('has_api_key')) ?? false
    bridge.value = (await call<BridgeInfo>('bridge_info')) ?? null
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
      stepText.value = null
      resultText.value = null
      plan.value = null
      thinking.value = null
      response.value = null
      steps.value = []
      await invoke('start_task', { text })
      return true
    } catch (err) {
      showNotice(String(err))
      return false
    }
  }

  const stop = () => call('stop')
  const toggleListening = () => call('toggle_listening')
  /** Answer the waiting question. A stale answer is refused by the Rust side. */
  async function answer(approved: boolean) {
    const waiting = question.value
    if (!waiting) return
    await call('confirm', { id: waiting.id, approved })
  }
  const debugSetState = (next: AgentState) => call('debug_set_state', { state: next })
  const debugCapture = (monitor?: number) => call<CaptureReport>('debug_capture', { monitor })
  const debugClick = (args: { x: number; y: number; delayMs?: number; monitor?: number }) =>
    call<ClickReport>('debug_click', args)
  const debugType = (text: string, delayMs?: number) => call('debug_type', { text, delayMs })

  // A task starts while the card is open: show its progress above the composer.
  watch(isRunning, (running) => {
    if (running && mode.value === 'open') void setMode('task')
  })

  /** Put the progress panel away and go back to the plain composer. */
  function clearTask() {
    plan.value = null
    thinking.value = null
    response.value = null
    steps.value = []
    if (mode.value === 'task') void setMode('open')
  }

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
      await listen<StepStartedPayload>(STEP_STARTED, (event) => {
        stepText.value = event.payload.text
        response.value = event.payload.text
        thinking.value = null
        steps.value = [
          ...steps.value,
          {
            step: event.payload.step,
            tool: event.payload.tool,
            detail: event.payload.detail,
            ok: null,
          },
        ]
      }),
      await listen<{ step: number; ok: boolean }>('step_finished', (event) => {
        steps.value = steps.value.map((s) =>
          s.step === event.payload.step ? { ...s, ok: event.payload.ok } : s,
        )
      }),
      await listen<TaskDonePayload>(TASK_DONE, (event) => {
        resultText.value = event.payload.summary
        response.value = event.payload.summary
        thinking.value = null
      }),
      await listen<PlanUpdatedPayload>(PLAN_UPDATED, (event) => (plan.value = event.payload)),
      await listen<ConfirmRequestPayload>(CONFIRM_REQUEST, (event) => {
        question.value = event.payload
        // The question needs room: open the card by itself.
        if (mode.value === 'ambient' || mode.value === 'open') void setMode('task')
      }),
      await listen<{ id: string }>(CONFIRM_RESOLVED, (event) => {
        if (question.value?.id === event.payload.id) question.value = null
      }),
      await listen<ThinkingPayload>(THINKING, (event) => (thinking.value = event.payload)),
      await listen<number>(AUDIO_LEVEL, (event) => {
        level.value = event.payload
        if (event.payload > SOUND_THRESHOLD) heardSound = true
      }),
      await listen<VoiceHeardPayload>(VOICE_HEARD, (event) => {
        const { text, note } = event.payload
        showNotice(text ? `${note}: “${text}”` : note)
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
    bridge,
    health,
    fitHeight,
    mode,
    plan,
    question,
    answer,
    thinking,
    response,
    steps,
    home,
    clearTask,
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
    debugCapture,
    debugClick,
    debugType,
  }
}

export type Agent = ReturnType<typeof useAgent>