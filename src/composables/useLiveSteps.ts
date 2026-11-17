import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onMounted, onBeforeUnmount, ref, shallowRef } from 'vue'
import { stepKey, type LiveStep } from '@/config/liveSteps'
import type { StepStartedPayload, StepFinishedPayload } from '@/types/protocol'
import type { ConfirmRequestPayload, ChoiceRecord } from '@/types/protocol'
import type { PlannedRequest, IndependentTask } from './useTaskBoard'

export interface LiveRunSnapshot {
  revision: number
  request: PlannedRequest | null
  tasks: IndependentTask[]
  steps: {
    start: StepStartedPayload
    finish: StepFinishedPayload | null
    startedAt: number
    finishedAt: number | null
  }[]
  droppedResults: number
  droppedSteps: number
  question?: ConfirmRequestPayload | null
  choices?: ChoiceRecord[]
}
export function startLiveStep(
  steps: LiveStep[],
  payload: StepStartedPayload,
  now: number,
): LiveStep[] {
  const taskId = payload.taskId ?? null
  if (steps.some((step) => stepKey(step) === stepKey({ ...payload, taskId }))) return steps
  const next: LiveStep[] = [
    ...steps,
    {
      ...payload,
      step: payload.step,
      taskId,
      tool: payload.tool,
      kind: payload.kind,
      model: payload.model,
      text: payload.text,
      detail: payload.detail,
      ok: null,
      startedAt: now,
      finishedAt: null,
    },
  ]
  while (next.length > 512) {
    const index = next.findIndex(
      (step) => !step.result && (step.kind === 'model' || step.ok !== null),
    )
    next.splice(index < 0 ? 0 : index, 1)
  }
  return next
}
export function finishLiveStep(
  steps: LiveStep[],
  payload: StepFinishedPayload,
  now: number,
): LiveStep[] {
  return steps.map((step) =>
    stepKey(step) ===
      stepKey({
        ...step,
        taskId: payload.taskId ?? null,
        requestId: payload.requestId,
        stepId: payload.stepId,
        step: payload.step,
      }) && step.ok === null
      ? {
          ...step,
          ok: payload.ok,
          model: payload.model ?? step.model,
          completionTokens: payload.completionTokens,
          tokensEstimated: payload.tokensEstimated,
          attempts: payload.attempts,
          error: payload.error,
          result: step.kind === 'tool' ? payload.result : undefined,
          finishedAt: now,
        }
      : step,
  )
}
export function snapshotLiveSteps(snapshot: LiveRunSnapshot): LiveStep[] {
  let steps: LiveStep[] = []
  for (const item of snapshot.steps) {
    steps = startLiveStep(steps, item.start, item.startedAt)
    if (item.finish) steps = finishLiveStep(steps, item.finish, item.finishedAt ?? item.startedAt)
  }
  return steps
}
export function useLiveSteps() {
  const steps = shallowRef<LiveStep[]>([])
  const snapshot = shallowRef<LiveRunSnapshot | null>(null)
  const question = shallowRef<ConfirmRequestPayload | null>(null)
  const choices = shallowRef<ChoiceRecord[]>([])
  const error = ref('')
  const offs: UnlistenFn[] = []
  let disposed = false
  let refreshing = false
  let again = false
  let buffered: (() => void)[] = []
  const clearTask = (id: number | null) => {
    steps.value = steps.value.filter((step) => step.taskId !== id)
  }
  async function refresh() {
    if (disposed) return
    if (refreshing) {
      again = true
      return
    }
    refreshing = true
    buffered = []
    try {
      const value = await invoke<LiveRunSnapshot>('live_run_snapshot')
      if (
        !disposed &&
        value &&
        Array.isArray(value.steps) &&
        value.revision >= (snapshot.value?.revision ?? -1)
      ) {
        snapshot.value = value
        question.value = value.question ?? null
        choices.value = value.choices ?? []
        steps.value = [
          ...steps.value.filter((step) => step.taskId === null && step.requestId == null),
          ...snapshotLiveSteps(value),
        ]
        for (const apply of buffered) apply()
        error.value = ''
      }
    } catch (cause) {
      if (!disposed) error.value = String(cause)
    } finally {
      refreshing = false
      if (again) {
        again = false
        void refresh()
      }
    }
  }
  function apply(action: () => void) {
    if (refreshing) buffered.push(action)
    action()
    void refresh()
  }
  onMounted(async () => {
    const subscribe = async <T>(name: string, handler: (payload: T) => void) => {
      const off = await listen<T>(name, (event) => {
        if (!disposed) handler(event.payload)
      })
      if (disposed) off()
      else offs.push(off)
    }
    try {
      await subscribe<StepStartedPayload>('step_started', (payload) => {
        const now = Date.now()
        apply(() => {
          steps.value = startLiveStep(steps.value, payload, now)
        })
      })
      await subscribe<StepFinishedPayload>('step_finished', (payload) => {
        const now = Date.now()
        apply(() => {
          steps.value = finishLiveStep(steps.value, payload, now)
        })
      })
      await subscribe<{ taskId?: number | null }>('task_started', (payload) =>
        apply(() => clearTask(payload?.taskId ?? null)),
      )
      await subscribe('tasks_updated', () => {
        void refresh()
      })
      await subscribe<ConfirmRequestPayload>('confirm_request', (payload) =>
        apply(() => {
          question.value = payload
        }),
      )
      await subscribe<{ id: string; choice?: string }>('confirm_resolved', (payload) =>
        apply(() => {
          const waiting = question.value
          if (waiting?.id === payload.id) {
            if (payload.choice && !choices.value.some((choice) => choice.id === payload.id))
              choices.value = [
                ...choices.value,
                {
                  id: payload.id,
                  taskId: waiting.taskId,
                  step: waiting.step,
                  label: payload.choice,
                  at: Date.now(),
                },
              ].slice(-512)
            question.value = null
          }
        }),
      )
      await refresh()
    } catch (cause) {
      if (!disposed) error.value = String(cause)
    }
  })
  onBeforeUnmount(() => {
    disposed = true
    offs.forEach((off) => off())
  })
  return { steps, snapshot, question, choices, error, clearTask, refresh }
}
export type LiveStepsModel = ReturnType<typeof useLiveSteps>