import type { OutcomeMetric, WorkspaceTask } from '@/types/workspace'
export function outcomeProgress(metric: OutcomeMetric | null | undefined): number | null {
  if (!metric || ![metric.baseline, metric.current, metric.target].every(Number.isFinite))
    return null
  if (metric.target === metric.baseline) return metric.current === metric.target ? 100 : 0
  return Math.round(
    Math.max(
      0,
      Math.min(1, (metric.current - metric.baseline) / (metric.target - metric.baseline)),
    ) * 100,
  )
}
export function planProgress(tasks: WorkspaceTask[]) {
  const latest = new Map(tasks.flatMap((t) => t.occurrences).map((o) => [o.workId ?? o.id, o]))
  const occurrences = [...latest.values()].filter((o) => o.state !== 'rescheduled')
  const done = occurrences.filter((o) => o.state === 'completed').length
  return {
    done,
    total: occurrences.length,
    value: occurrences.length ? Math.round((done / occurrences.length) * 100) : null,
  }
}
export function taskProgress(task: WorkspaceTask): number | null {
  const current = task.occurrences.at(-1)
  if (current?.state === 'completed') return 100
  if (task.executionMode === 'manual') return 0
  return task.totalSteps > 0 ? Math.round((task.completedSteps / task.totalSteps) * 100) : null
}