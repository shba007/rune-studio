export interface TaskTiming {
  startedAt: number
  goalStartedAt: number
  doneGoals: number
}

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