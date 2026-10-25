import type { IndependentTask, PlannedRequest } from '@/composables/useTaskBoard'
export const isTaskActive = (phase: string) => ['queued', 'planning', 'running'].includes(phase)
export function visibleGoal(requests: PlannedRequest[]): PlannedRequest | undefined {
  return (
    requests.find((r) => r.phase === 'running' || r.phase === 'planning') ??
    requests.find((r) => r.phase === 'queued') ??
    requests.at(-1)
  )
}
export function currentStep(task: IndependentTask): number {
  if (task.currentStep !== undefined)
    return Math.min(task.steps.length, Math.max(0, task.currentStep))
  if (task.phase === 'done') return task.steps.length
  const running = task.steps.findIndex((s) => s.phase === 'running' || s.phase === 'failed')
  return running >= 0 ? running + 1 : task.steps.filter((s) => s.phase === 'done').length
}
export function taskElapsed(task: IndependentTask, now: number): number {
  return task.startedAt == null ? 0 : Math.max(0, (task.finishedAt ?? now) - task.startedAt)
}
export function taskPosition(tasks: IndependentTask[]): string {
  const running = tasks.flatMap((task, i) => (task.phase === 'running' ? [i + 1] : []))
  const numerator = running.length
    ? running.join(', ')
    : String(tasks.filter((t) => t.phase === 'done').length)
  return `${numerator}/${tasks.length}`
}
// Estimated remaining execution time from observed, completed steps; never invent an initial duration.
export function estimateRemaining(tasks: IndependentTask[], now: number): number | null {
  const remaining = tasks.filter((t) => isTaskActive(t.phase))
  if (!remaining.length) return 0
  let completedMs = 0
  let completedSteps = 0
  for (const task of tasks) {
    if (task.startedAt == null) continue
    if (task.phase === 'done' && task.finishedAt != null) {
      completedMs += Math.max(0, task.finishedAt - task.startedAt)
      completedSteps += task.steps.length
    } else if (task.stepStartedAt != null && currentStep(task) > 1) {
      completedMs += Math.max(0, task.stepStartedAt - task.startedAt)
      completedSteps += currentStep(task) - 1
    }
  }
  if (!completedSteps || completedMs <= 0) return null
  const average = completedMs / completedSteps
  let background = 0
  let desktop = 0
  let longest = 0
  for (const task of remaining) {
    const done = task.steps.filter((s) => s.phase === 'done').length
    const elapsed =
      task.phase === 'running' && task.stepStartedAt != null
        ? Math.max(0, now - task.stepStartedAt)
        : 0
    const left = average * (task.steps.length - done) - elapsed
    if (left <= 0) return null // Re-estimate after new progress, instead of showing a false zero.
    longest = Math.max(longest, left)
    if (task.kind === 'desktop') desktop += left
    else background += left
  }
  return Math.max(longest, desktop, background / 2)
}