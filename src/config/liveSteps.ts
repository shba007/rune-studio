import type { StepStartedPayload, StepFinishedPayload } from '@/types/protocol'
import type { IconName } from '@/icons'

export type LiveStep = Pick<
  StepStartedPayload,
  | 'step'
  | 'taskId'
  | 'tool'
  | 'kind'
  | 'model'
  | 'text'
  | 'detail'
  | 'stepId'
  | 'requestId'
  | 'purpose'
> & {
  completionTokens?: number
  tokensEstimated?: boolean
  attempts?: number
  ok: boolean | null
  startedAt?: number
  finishedAt?: number | null
  result?: StepFinishedPayload['result']
  error?: string | null
}

export const STEP_KIND: Record<LiveStep['kind'], { label: string; icon: IconName }> = {
  model: { label: 'Model call', icon: 'thinking' },
  tool: { label: 'Tool call', icon: 'plug' },
  screen: { label: 'Screen action', icon: 'mouse' },
}

// A Kit tool is named "<server>__<tool>" (the agent loop adds a "kit__" prefix). The card
// shows the tool on the title line and the server beside the kind, like the Playground.
export function stepTitle(step: LiveStep): { title: string; server: string | null } {
  const text = step.text || step.tool
  if (step.kind !== 'tool') return { title: text, server: null }
  const parts = text.replace(/^kit__/, '').split('__')
  if (parts.length < 2 || parts.some((part) => !part || /\s/.test(part)))
    return { title: text, server: null }
  return { title: parts.slice(1).join('__'), server: parts[0]! }
}

export const stepKey = (step: Pick<LiveStep, 'taskId' | 'requestId' | 'stepId' | 'step'>) =>
  `${step.taskId ?? (step.requestId != null ? `request:${step.requestId}` : 'run')}:${step.stepId ?? step.step}`

export const stepStatus = (step: LiveStep) =>
  step.ok === null ? 'Running' : step.ok ? 'Done' : 'Failed'

export function stepDuration(step: LiveStep, now: number): number | null {
  if (step.startedAt === undefined) return null
  return Math.max(0, (step.finishedAt ?? now) - step.startedAt)
}

export function durationText(ms: number): string {
  if (ms < 1000) return '<1 s'
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`
  const total = Math.round(ms / 1000)
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`
}

export interface LaneBlock {
  step: LiveStep
  left: number
  width: number
}

export function laneLayout(steps: LiveStep[], now: number) {
  const timed = steps.filter((step) => step.startedAt !== undefined)
  if (!timed.length) return { span: 0, blocks: [] as LaneBlock[] }
  const start = Math.min(...timed.map((step) => step.startedAt!))
  const end = Math.max(...timed.map((step) => step.finishedAt ?? Math.max(now, step.startedAt!)))
  const span = Math.max(1000, end - start)
  const blocks = timed.map((step) => ({
    step,
    left: ((step.startedAt! - start) / span) * 100,
    width: (((step.finishedAt ?? Math.max(now, step.startedAt!)) - step.startedAt!) / span) * 100,
  }))
  return { span, blocks }
}
export const kindColor = (kind: LiveStep['kind']) =>
  kind === 'model' ? 'text-model' : kind === 'tool' ? 'text-tool' : 'text-voice'
export const modelUsage = (step: LiveStep) =>
  [
    step.completionTokens == null
      ? ''
      : `${step.tokensEstimated ? '~' : ''}${step.completionTokens} tokens`,
    (step.attempts ?? 0) > 1 ? `${step.attempts} attempts` : '',
  ]
    .filter(Boolean)
    .join(' · ')
export const actionSteps = (steps: LiveStep[]) => steps.filter((step) => step.kind !== 'model')