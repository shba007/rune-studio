import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { afterEach, expect, it, vi } from 'vite-plus/test'
import {
  useLiveSteps,
  startLiveStep,
  finishLiveStep,
  snapshotLiveSteps,
  type LiveRunSnapshot,
  type LiveStepsModel,
} from './useLiveSteps'
import type { StepStartedPayload } from '@/types/protocol'
const harness = vi.hoisted(() => ({
  invoke: vi.fn<(command: string) => Promise<unknown>>(),
  handlers: new Map<string, Set<(event: { payload: unknown }) => void>>(),
}))
vi.mock('@tauri-apps/api/core', () => ({ invoke: harness.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
    const set = harness.handlers.get(name) ?? new Set()
    set.add(handler)
    harness.handlers.set(name, set)
    return () => set.delete(handler)
  },
}))
afterEach(() => {
  harness.handlers.clear()
  harness.invoke.mockReset()
  vi.restoreAllMocks()
})
const start: StepStartedPayload = {
  taskId: 1,
  step: 1,
  kind: 'tool',
  model: 'local',
  tool: 'kit_tool',
  text: 'Time',
  detail: 'time__now',
  reason: '',
  point: null,
  double: false,
  button: 'left',
  durationMs: 0,
}
const finish = { taskId: 1, step: 1, ok: true, result: { text: 'result' } }
const snapshot: LiveRunSnapshot = {
  revision: 1,
  request: null,
  tasks: [],
  droppedResults: 0,
  droppedSteps: 0,
  steps: [{ start, finish: null, startedAt: 1000, finishedAt: null }],
}
function windowState() {
  let live!: LiveStepsModel
  const wrapper = mount(
    defineComponent({
      setup() {
        live = useLiveSteps()
        return () => null
      },
    }),
  )
  return {
    wrapper,
    get live() {
      return live
    },
  }
}
const send = (name: string, payload: unknown) =>
  harness.handlers.get(name)?.forEach((handler) => handler({ payload }))

it('snapshot then buffered events agrees with events alone and ignores unknown finishes', async () => {
  let resolve!: (value: LiveRunSnapshot) => void
  harness.invoke
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done
        }),
    )
    .mockResolvedValue(null)
  const state = windowState()
  await flushPromises()
  vi.spyOn(Date, 'now').mockReturnValue(2000)
  send('step_finished', finish)
  resolve(snapshot)
  await flushPromises()
  const alone = finishLiveStep(startLiveStep([], start, 1000), finish, 2000)
  expect(state.live.steps.value).toEqual(alone)
  expect(
    snapshotLiveSteps({
      ...snapshot,
      steps: [{ start, finish, startedAt: 1000, finishedAt: 2000 }],
    }),
  ).toEqual(alone)
  expect(finishLiveStep([], finish, 2000)).toEqual([])
  state.wrapper.unmount()
})

it('keeps two windows consistent, deduplicates starts, and clears dismissed snapshot data', async () => {
  harness.invoke.mockResolvedValue(null)
  vi.spyOn(Date, 'now').mockReturnValue(1000)
  const first = windowState(),
    second = windowState()
  await flushPromises()
  send('step_started', start)
  send('step_started', start)
  send('step_finished', finish)
  await flushPromises()
  expect(first.live.steps.value).toEqual(second.live.steps.value)
  expect(first.live.steps.value).toHaveLength(1)
  harness.invoke.mockResolvedValue({ ...snapshot, revision: 10, steps: [] })
  send('tasks_updated', {})
  await flushPromises()
  expect(first.live.steps.value).toEqual([])
  expect(second.live.steps.value).toEqual([])
  first.wrapper.unmount()
  second.wrapper.unmount()
})
it('pairs model and action with the same number using IDs for both events and snapshots', () => {
  const model = {
    ...start,
    kind: 'model' as const,
    stepId: 'model:1',
    requestId: 9,
    purpose: 'Decide',
  }
  const modelFinish = {
    taskId: 1,
    requestId: 9,
    stepId: 'model:1',
    step: 88,
    ok: true,
    model: 'served',
    completionTokens: 12,
    tokensEstimated: false,
    attempts: 2,
  }
  let events = startLiveStep(startLiveStep([], model, 100), start, 200)
  events = finishLiveStep(events, modelFinish, 300)
  expect(events[0]).toMatchObject({ ok: true, model: 'served', completionTokens: 12, attempts: 2 })
  expect(events[1]!.ok).toBeNull()
  events = finishLiveStep(events, finish, 400)
  expect(events[1]!.result).toEqual(finish.result)
  expect(
    snapshotLiveSteps({
      ...snapshot,
      steps: [
        { start: model, finish: modelFinish, startedAt: 100, finishedAt: 300 },
        { start, finish, startedAt: 200, finishedAt: 400 },
      ],
    }),
  ).toEqual(events)
})

it('keeps the tool result when metadata-only model events exceed the bound', () => {
  let steps = finishLiveStep(startLiveStep([], start, 1), finish, 2)
  for (let i = 0; i < 540; i++) {
    steps = startLiveStep(steps, { ...start, kind: 'model', stepId: 'model:' + i }, i + 3)
  }
  expect(steps).toHaveLength(512)
  expect(steps[0]!.result).toEqual(finish.result)
})
it('accepts the newest tool result when all retained steps already have results', () => {
  let steps = Array.from({ length: 512 }, (_, i) => ({
    ...start,
    step: i + 1,
    ok: true,
    result: { text: 'old' },
    startedAt: i,
    finishedAt: i + 1,
  }))
  const pending = { ...start, step: 513 }
  steps = finishLiveStep(
    startLiveStep(steps, pending, 600),
    { ...finish, step: 513, result: { text: 'new' } },
    700,
  ) as typeof steps
  expect(steps).toHaveLength(512)
  expect(steps.at(-1)!.result.text).toBe('new')
})