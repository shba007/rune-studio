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

/** A short, friendly time: "any moment", "~40 s", "~2 min", "~1 h 5 min". */
export function etaText(ms: number): string {
  const seconds = Math.ceil(ms / 1000)
  if (seconds <= 3) return 'any moment'
  if (seconds < 60) return `~${Math.ceil(seconds / 5) * 5} s`
  const minutes = Math.ceil(seconds / 60)
  if (minutes < 60) return `~${minutes} min`
  const hours = Math.floor(minutes / 60)
  const rest = minutes % 60
  return rest === 0 ? `~${hours} h` : `~${hours} h ${rest} min`
}