import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTimeoutFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { REVIEW_SECONDS, readOffer, type ReviewOffer } from '@/config/review'
import {
  AUDIO_LEVEL,
  VOICE_HEARD,
  OPEN_SETTINGS,
  CONFIRM_REQUEST,
  CONFIRM_RESOLVED,
  PLAN_UPDATED,
  REFLECTION_PROGRESS,
  REFLECTION_PROMPT,
  REFLECTION_RESOLVED,
  STATE_CHANGED,
  STEP_STARTED,
  SUMMON,
  TASK_DONE,
  TASK_VERIFIED,
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
  type ReflectionProgressPayload,
  type ReflectionPromptPayload,
  type Settings,
  type StateChangedPayload,
  type StepStartedPayload,
  type TaskDonePayload,
  type TaskVerifiedPayload,
  type ThinkingPayload,
} from '@/types/protocol'
import { friendly, noticeMillis } from '@/config/errors'

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
const MODE_RANK: Record<PillMode, number> = {
  ambient: 0,
  open: 1,
  task: 2,
  analysis: 2,
  settings: 3,
}

/** Longer than the page and width transitions, so the window only shrinks once they have finished. */
const COLLAPSE_MS = 340
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
  /** An error (red) or plain information (a confirmation such as "Saved"). */
  const noticeKind = ref<'error' | 'info'>('error')
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

  /** The idle review offer, while it is up; `endsAt` is when the countdown runs out on this window's clock. */
  const reflectionPrompt = ref<{ endsAt: number } | null>(null)
  /** How far the idle review is (null when none is running). */
  const reflection = ref<ReflectionProgressPayload | null>(null)

  /** The offer to review a finished session, while it is up (`endsAt` on this window's clock). */
  const reviewOffer = ref<(ReviewOffer & { endsAt: number }) | null>(null)

  const unlisten: UnlistenFn[] = []
  let modeToken = 0
  let heardSound = false

  let noticeHandle: ReturnType<typeof setTimeout> | undefined
  /** The content's height as last reported, so a mode change can restore it. */
  let wantedHeight = 0
  const silenceTimer = useTimeoutFn(
    () => {
      if (!heardSound) showNotice('No sound from the microphone. Check Windows microphone access')
    },
    SILENCE_MS,
    { immediate: false },
  )

  /** A newer notice always gets its full time on screen; a longer one stays longer. */
  function showNotice(message: string, kind: 'error' | 'info' = 'error') {
    const text = friendly(message)
    notice.value = text
    noticeKind.value = kind
    clearTimeout(noticeHandle)
    noticeHandle = setTimeout(() => (notice.value = null), noticeMillis(text))
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
    // A question waits for its answer: the card stays up until it is answered (or times out).
    if (next === 'ambient' && question.value) return
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
    // set_mode gives the window the page's default height. The content may have settled at another height
    // while it waited (the page animates before the window shrinks), so ask for the content's height again.
    if (token === modeToken && wantedHeight >= 48) await fitHeight(wantedHeight)
    if (next === 'open') focusTick.value += 1
  }

  /** Open when collapsed; collapse when open or in settings. */
  /** Make the window as tall as the open card's content. Only the open and task pages do this. */
  async function fitHeight(height: number) {
    // Only pages that fit their content count: the settings page has a fixed height of its own.
    if (mode.value === 'open' || mode.value === 'task' || mode.value === 'analysis') {
      wantedHeight = height
      await call('fit_height', { height })
    }
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
  const dismissReview = () => (reviewOffer.value = null)
  /** Opens the Flight Player on this session and puts the card away. */
  async function openReview(runId: string) {
    reviewOffer.value = null
    await call('open_flight', { runId })
  }
  /** Start (true) or Deny (false) the idle review. */
  async function answerReflection(start: boolean) {
    reflectionPrompt.value = null
    await call('reflection_answer', { start })
  }
  /** Tells the backend the pill is being used, so the idle clock restarts (at most every 5 s). */
  let lastActivity = 0
  function noteActivity() {
    const now = Date.now()
    if (now - lastActivity < 5000) return
    lastActivity = now
    void call('pill_activity')
  }
  /** Looks at the screen and shows the answer in the analysis window (a refusal shows as a notice). */
  const analyzeScreen = () => call('analyze_screen', { question: null })
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
      // Only a contradiction is worth interrupting for; verified, unsure and errors stay in the trace.
      await listen<TaskVerifiedPayload>(TASK_VERIFIED, (event) => {
        if (event.payload.result === 'contradicted')
          showNotice(`Check: the screen may not show the task as done: ${event.payload.evidence}`)
      }),
      await listen<ReflectionPromptPayload>(REFLECTION_PROMPT, (event) => {
        reflectionPrompt.value = { endsAt: Date.now() + event.payload.endsInMs }
        // The offer needs room: open the card by itself, as an update prompt does.
        if (mode.value === 'ambient') void setMode('open')
      }),
      await listen(REFLECTION_RESOLVED, () => (reflectionPrompt.value = null)),
      await listen<ReflectionProgressPayload>(REFLECTION_PROGRESS, (event) => {
        // The ring shows the plan or the review, never both: a review has no plan.
        reflection.value = event.payload.total > 0 ? event.payload : null
        if (event.payload.total > 0) plan.value = null
      }),
      await listen('review_offer', (event) => {
        const offer = readOffer(event.payload)
        // Never over a running task; the confirm card and the idle prompt take the place first.
        if (offer && !isRunning.value) {
          reviewOffer.value = { ...offer, endsAt: Date.now() + REVIEW_SECONDS * 1000 }
          if (mode.value === 'ambient') void setMode('open')
        }
      }),
      await listen<PlanUpdatedPayload>(PLAN_UPDATED, (event) => (plan.value = event.payload)),
      await listen<ConfirmRequestPayload>(CONFIRM_REQUEST, (event) => {
        question.value = event.payload
        // The question needs room: open the card by itself, also over the settings page.
        if (mode.value !== 'task' && mode.value !== 'analysis') void setMode('task')
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
    noticeKind,
    notify: showNotice,
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
    reflectionPrompt,
    reviewOffer,
    dismissReview,
    openReview,
    reflection,
    answerReflection,
    noteActivity,
    analyzeScreen,
    toggleListening,
    confirm,
    debugSetState,
    debugCapture,
    debugClick,
    debugType,
  }
}

export type Agent = ReturnType<typeof useAgent>