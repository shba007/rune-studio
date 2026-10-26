import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useIntervalFn, useTimeoutFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { TaskTiming } from '@/config/eta'
import { REVIEW_SECONDS, readOffer, type ReviewOffer } from '@/config/review'
import {
  AUDIO_LEVEL,
  VOICE_HEARD,
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
  type BackendProbe,
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
  type ScreenRequestSnapshot,
  type StateChangedPayload,
  type StepStartedPayload,
  type TaskDonePayload,
  type TaskVerifiedPayload,
  type ThinkingPayload,
} from '@/types/protocol'
import { friendly, noticeMillis } from '@/config/errors'

const RUNNING: AgentState[] = [
  'planning',
  'awaiting_confirm',
  'acting',
  'observing',
  'loading_model',
]

const STATE_LABELS: Record<AgentState, string> = {
  idle: 'Ready',
  listening: 'Listening',
  planning: 'Thinking',
  awaiting_confirm: 'Needs your OK',
  acting: 'Acting',
  observing: 'Looking',
  reflecting: 'Reflecting',
  loading_model: 'Loading model',
  done: 'Done',
  failed: 'Failed',
  stopped: 'Stopped',
}

const MODE_RANK: Record<PillMode, number> = {
  ambient: 0,
  open: 1,
  task: 2,
  analysis: 2,
}

const COLLAPSE_MS = 340
const SILENCE_MS = 4000
const SOUND_THRESHOLD = 0.04

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

export function useAgent() {
  const screenRequest = ref<ScreenRequestSnapshot>({ id: null, status: 'idle', waiting: false })
  const state = ref<AgentState>('idle')
  const backend = ref<BackendStatus | null>(null)
  let backendRevision = 0
  let backendChecks = 0
  function applyBackend(probe: BackendProbe) {
    if (probe && Number.isFinite(probe.revision) && probe.revision >= backendRevision) {
      backendRevision = probe.revision
      backend.value = probe.status
    }
  }
  const settings = ref<Settings | null>(null)
  const hasApiKey = ref(false)
  const bridge = ref<BridgeInfo | null>(null)
  const mode = ref<PillMode>('ambient')
  const plan = ref<PlanUpdatedPayload | null>(null)
  const timing = ref<TaskTiming | null>(null)
  const thinking = ref<ThinkingPayload | null>(null)
  const response = ref<string | null>(null)
  const question = ref<ConfirmRequestPayload | null>(null)
  const steps = ref<{ step: number; tool: string; detail: string; ok: boolean | null }[]>([])
  const notice = ref<string | null>(null)
  const noticeKind = ref<'error' | 'info'>('error')
  const focusTick = ref(0)
  const level = ref(0)

  const isRunning = computed(() => RUNNING.includes(state.value))

  const health = computed<{ tone: 'ok' | 'warn' | 'down' | 'checking'; title: string }>(() => {
    if (settings.value?.externalAgent)
      return { tone: 'ok', title: 'External agent mode — use your host to start a task' }
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
  const stepText = ref<string | null>(null)
  const resultText = ref<string | null>(null)
  const statusText = computed(() => {
    if (notice.value) return notice.value
    if (isRunning.value && stepText.value) return stepText.value
    if ((state.value === 'done' || state.value === 'failed') && resultText.value)
      return resultText.value
    return STATE_LABELS[state.value]
  })

  const reflectionPrompt = ref<{ endsAt: number } | null>(null)
  const reflection = ref<ReflectionProgressPayload | null>(null)

  const reviewOffer = ref<(ReviewOffer & { endsAt: number }) | null>(null)
  const reviewStage = ref<'complete' | 'why' | 'review'>('complete')

  const unlisten: UnlistenFn[] = []
  let modeToken = 0
  let heardSound = false

  let noticeHandle: ReturnType<typeof setTimeout> | undefined
  let wantedHeight = 0
  let fitting: Promise<void> | undefined
  let fittedHeight = 0
  const silenceTimer = useTimeoutFn(
    () => {
      if (!heardSound) showNotice('No sound from the microphone. Check Windows microphone access')
    },
    SILENCE_MS,
    { immediate: false },
  )

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

  async function setMode(next: PillMode) {
    if (next === 'ambient' && question.value) return
    const token = ++modeToken
    if (fitting) await fitting
    fittedHeight = 0
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
    if (token === modeToken && wantedHeight >= 48) await fitHeight(wantedHeight)
    if (next === 'open') focusTick.value += 1
  }

  async function fitHeight(height: number) {
    if (!Number.isFinite(height)) return
    if (mode.value === 'open' || mode.value === 'task' || mode.value === 'analysis') {
      wantedHeight = Math.max(64, Math.ceil(height))
      if (!fitting) {
        fitting = (async () => {
          // Collect measurements from the same render before crossing IPC.
          await Promise.resolve()
          while (mode.value !== 'ambient' && fittedHeight !== wantedHeight) {
            const next = wantedHeight
            await call('fit_height', { height: next })
            fittedHeight = next
          }
        })().finally(() => {
          fitting = undefined
        })
      }
      await fitting
    }
  }

  const home = (): PillMode => (plan.value ? 'task' : 'open')
  const toggleOpen = () => setMode(mode.value === 'ambient' ? home() : 'ambient')
  const dismiss = () => (mode.value === 'ambient' ? Promise.resolve() : setMode('ambient'))

  async function refreshBackend() {
    backendChecks += 1
    try {
      const probe = await call<BackendProbe>('check_backend')
      if (probe) applyBackend(probe)
    } finally {
      backendChecks -= 1
    }
  }

  useIntervalFn(() => {
    if (
      backendChecks === 0 &&
      !new URLSearchParams(location.search).get('view') &&
      settings.value &&
      !settings.value.externalAgent
    )
      void refreshBackend()
  }, 15000)

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

  function resetTaskView() {
    stepText.value = null
    resultText.value = null
    plan.value = null
    timing.value = null
    reviewOffer.value = null
    thinking.value = null
    response.value = null
    steps.value = []
  }

  async function startTask(text: string): Promise<boolean> {
    try {
      resetTaskView()
      await invoke('start_task', { text })
      return true
    } catch (err) {
      showNotice(String(err))
      return false
    }
  }

  const stop = () => call('stop')
  const dismissReview = () => (reviewOffer.value = null)
  function goReview() {
    reviewStage.value = 'review'
    if (reviewOffer.value)
      reviewOffer.value = { ...reviewOffer.value, endsAt: Date.now() + REVIEW_SECONDS * 1000 }
  }
  async function answerComplete(complete: boolean | null) {
    const offer = reviewOffer.value
    if (!offer || complete === null) return goReview()
    if (complete) {
      await call('session_feedback', { runId: offer.runId, complete: true, reason: null })
      goReview()
    } else {
      reviewStage.value = 'why'
    }
  }
  async function answerWhy(reason: string) {
    const offer = reviewOffer.value
    if (offer)
      await call('session_feedback', {
        runId: offer.runId,
        complete: false,
        reason: reason.trim() || null,
      })
    goReview()
  }
  async function openReview(runId: string) {
    reviewOffer.value = null
    await call('open_flight', { runId })
  }
  async function answerReflection(start: boolean) {
    reflectionPrompt.value = null
    await call('reflection_answer', { start })
  }
  let lastActivity = 0
  function noteActivity() {
    const now = Date.now()
    if (now - lastActivity < 5000) return
    lastActivity = now
    void call('pill_activity')
  }
  const refreshScreenRequest = async () => {
    const snapshot = await call<ScreenRequestSnapshot>('screen_request_state')
    if (snapshot) screenRequest.value = snapshot
  }
  const analyzeScreen = async () => {
    if (settings.value?.externalAgent) {
      await call('analyze_screen', { question: null })
      await refreshScreenRequest()
    } else {
      await setMode('analysis')
    }
  }
  const cancelScreenRequest = () => call('cancel_screen_request')
  useIntervalFn(() => {
    if (
      settings.value?.externalAgent &&
      ['queued', 'delivered'].includes(screenRequest.value.status)
    )
      void refreshScreenRequest()
  }, 1000)
  const openSettings = (section = 'general') => call('open_settings_window', { section })
  const openWorkspace = (section = 'tasks') => call('open_workspace', { section })
  const toggleListening = () => call('toggle_listening')
  async function answer(approved: boolean) {
    const waiting = question.value
    if (!waiting) return
    await call('confirm', { id: waiting.id, approved })
  }
  const debugSetState = (next: AgentState) => call('debug_set_state', { state: next })
  const debugCapture = (monitor?: number) => call<CaptureReport>('debug_capture', { monitor })
  const debugClick = (args: { x: number; y: number; delayMs?: number; monitor?: number }) =>
    call<ClickReport>('debug_click', args)
  const debugMove = (x: number, y: number, delayMs?: number) =>
    call('debug_move', { x, y, delayMs })
  const debugType = (text: string, delayMs?: number) => call('debug_type', { text, delayMs })

  watch(isRunning, (running) => {
    if (running && !timing.value) {
      const now = Date.now()
      timing.value = { startedAt: now, goalStartedAt: now, doneGoals: 0 }
    }
    if (running && mode.value === 'open') void setMode('task')
  })

  watch(
    () => (plan.value ? (plan.value.finished ? plan.value.goals.length : plan.value.current) : 0),
    (done) => {
      if (timing.value && done !== timing.value.doneGoals)
        timing.value = { ...timing.value, goalStartedAt: Date.now(), doneGoals: done }
    },
  )

  function clearTask() {
    plan.value = null
    timing.value = null
    thinking.value = null
    response.value = null
    steps.value = []
    if (mode.value === 'task') void setMode('open')
  }

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
      await listen<BackendProbe>('backend_status', (event) => applyBackend(event.payload)),
    )
    unlisten.push(
      await listen('task_started', resetTaskView),
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
      await listen<TaskVerifiedPayload>(TASK_VERIFIED, (event) => {
        if (event.payload.result === 'contradicted')
          showNotice(`Check: the screen may not show the task as done: ${event.payload.evidence}`)
      }),
      await listen<ReflectionPromptPayload>(REFLECTION_PROMPT, (event) => {
        reflectionPrompt.value = { endsAt: Date.now() + event.payload.endsInMs }
        if (mode.value === 'ambient') void setMode('open')
      }),
      await listen(REFLECTION_RESOLVED, () => (reflectionPrompt.value = null)),
      await listen<ReflectionProgressPayload>(REFLECTION_PROGRESS, (event) => {
        reflection.value = event.payload.total > 0 ? event.payload : null
        if (event.payload.total > 0) plan.value = null
      }),
      await listen('review_offer', (event) => {
        const offer = readOffer(event.payload)
        if (offer && !isRunning.value) {
          reviewOffer.value = { ...offer, endsAt: Date.now() + REVIEW_SECONDS * 1000 }
          reviewStage.value = 'complete'
          if (mode.value === 'ambient') void setMode('open')
        }
      }),
      await listen<PlanUpdatedPayload>(PLAN_UPDATED, (event) => (plan.value = event.payload)),
      await listen<ConfirmRequestPayload>(CONFIRM_REQUEST, (event) => {
        question.value = event.payload
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
      await listen<ScreenRequestSnapshot>('screen_request_updated', (event) => {
        screenRequest.value = event.payload
      }),
      await listen('settings_changed', () => {
        void refreshSettings()
        void refreshBackend()
      }),
    )
    await refreshSettings()
    await refreshScreenRequest()
    await refreshBackend()
  })

  onBeforeUnmount(() => unlisten.forEach((off) => off()))

  return {
    cancelScreenRequest,
    screenRequest,
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
    timing,
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
    reviewStage,
    answerComplete,
    answerWhy,
    dismissReview,
    openReview,
    reflection,
    answerReflection,
    noteActivity,
    analyzeScreen,
    openSettings,
    openWorkspace,
    toggleListening,
    confirm,
    debugSetState,
    debugCapture,
    debugClick,
    debugMove,
    debugType,
  }
}

export type Agent = ReturnType<typeof useAgent>