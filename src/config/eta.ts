/** When the task started, and when the goal being worked on started (ms, on the window's clock). */
export interface TaskTiming {
  startedAt: number
  goalStartedAt: number
  /** Goals finished when `goalStartedAt` was taken. */
  doneGoals: number
}

/**
 * Time left, from how long the finished goals took on average. Null until one goal is done (there is
 * nothing to go on before that). The time already spent on the current goal is taken off, so as the
 * clock moves this counts down; it stops at 0 when a goal takes longer than the others did.
 */
export function remainingMs(
  timing: TaskTiming,
  goals: number,
  doneGoals: number,
  now: number,
): number | null {
  if (goals <= 0 || doneGoals <= 0 || doneGoals >= goals) return null
  const average = (timing.goalStartedAt - timing.startedAt) / doneGoals
  return Math.max(0, average * (goals - doneGoals) - (now - timing.goalStartedAt))
}

export function clockText(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = total % 60
  const pair = (n: number) => String(n).padStart(2, '0')
  return hours > 0 ? `${hours}:${pair(minutes)}:${pair(seconds)}` : `${minutes}:${pair(seconds)}`
}