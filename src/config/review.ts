export const REVIEW_SECONDS = 15

export interface ReviewOffer {
  runId: string
  task: string
  outcome: 'succeeded' | 'failed' | 'stopped'
  steps: number
  durationSecs: number
  source: string
}

export function reviewMeta(durationSecs: number, steps: number): string {
  const seconds = Math.max(0, Math.round(durationSecs))
  const clock = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
  return `${clock} · ${steps} ${steps === 1 ? 'STEP' : 'STEPS'}`
}

export function dotsLeft(now: number, endsAt: number, total = REVIEW_SECONDS): number {
  return Math.min(total, Math.max(0, Math.ceil((endsAt - now) / 1000)))
}

export function readOffer(raw: unknown): ReviewOffer | null {
  if (typeof raw !== 'object' || raw === null) return null
  const value = raw as Record<string, unknown>
  if (typeof value.runId !== 'string' || value.runId === '') return null
  const outcome =
    value.outcome === 'succeeded' || value.outcome === 'stopped' ? value.outcome : 'failed'
  const number = (item: unknown) => (typeof item === 'number' && Number.isFinite(item) ? item : 0)
  return {
    runId: value.runId,
    task: typeof value.task === 'string' ? value.task : '',
    outcome,
    steps: number(value.steps),
    durationSecs: number(value.durationSecs),
    source: typeof value.source === 'string' ? value.source : 'Rune',
  }
}