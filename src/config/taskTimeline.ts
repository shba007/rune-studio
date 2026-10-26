export interface TaskLane {
  id: number
  title: string
  kind: 'desktop' | 'background'
  phase: string
  runId: string | null
  start: number
  end: number
  steps: { title: string; phase: string; start: number; end: number }[]
}
export interface TaskTimeline {
  origin: number
  total: number
  requestId: string
  rows: TaskLane[]
}
const object = (value: unknown): Record<string, unknown> =>
  value !== null && typeof value === 'object' ? (value as Record<string, unknown>) : {}
const timestamp = (value: unknown): number | null =>
  typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null
const text = (value: unknown) => (typeof value === 'string' ? value : '')

export function taskTimeline(raw: unknown): TaskTimeline | null {
  if (!Array.isArray(raw) || !raw.length) return null
  const history = raw
    .map(object)
    .filter((item) => timestamp(item.atMs) !== null && timestamp(item.revision) !== null)
    .sort((a, b) => Number(a.revision) - Number(b.revision))
  const latest = history.at(-1)
  if (!latest) return null
  const snapshot = object(latest.snapshot)
  const request = object(snapshot.request)
  if (!Array.isArray(snapshot.tasks) || !snapshot.tasks.length) return null
  const origin = Math.min(...history.map((item) => Number(item.atMs)))
  const end = Math.max(
    Number(latest.atMs),
    timestamp(request.finishedAt) ?? 0,
    ...snapshot.tasks.map((task) => timestamp(object(task).finishedAt) ?? 0),
  )
  const total = Math.max(1, end - origin)
  const offset = (at: unknown, fallback: number) =>
    Math.max(0, Math.min(total, (timestamp(at) ?? fallback) - origin))
  const rows = snapshot.tasks
    .map(object)
    .filter((task) => timestamp(task.id) !== null)
    .map((task) => {
      const start = offset(task.startedAt, timestamp(task.finishedAt) ?? end)
      const finish = offset(task.finishedAt, end)
      const steps = Array.isArray(task.steps)
        ? task.steps
            .map(object)
            .filter((step) => timestamp(step.startedAt) !== null)
            .map((step) => ({
              title: text(step.text),
              phase: text(step.phase),
              start: offset(step.startedAt, origin),
              end: offset(step.finishedAt, timestamp(task.finishedAt) ?? end),
            }))
        : []
      return {
        id: Number(task.id),
        title: text(task.text),
        kind: task.kind === 'desktop' ? ('desktop' as const) : ('background' as const),
        phase: text(task.phase),
        runId: text(task.runId) || null,
        start,
        end: Math.max(start, finish),
        steps,
      }
    })
  return rows.length ? { origin, total, requestId: text(request.historyId), rows } : null
}