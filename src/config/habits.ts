export interface Habit {
  id: string
  title: string
  targetMinutes: number | null
  state: string
  dueMs: number | null
  completedMs: number | null
  creditedMs: number
}

export interface HabitChip {
  id: string
  title: string
  kind: 'check' | 'timed'
  done: boolean
  label: string
}

const MINUTE = 60_000

export const sameDay = (ms: number, now: number) =>
  new Date(ms).toDateString() === new Date(now).toDateString()

export function todaysChips(habits: Habit[], now: number): HabitChip[] {
  const chips: HabitChip[] = []
  for (const habit of habits) {
    const todo = habit.state === 'pending' || habit.state === 'in_progress'
    const done =
      habit.state === 'completed' && habit.completedMs !== null && sameDay(habit.completedMs, now)
    if (!todo && !done) continue
    const timed = habit.targetMinutes !== null
    const minutes = Math.floor(habit.creditedMs / MINUTE)
    chips.push({
      id: habit.id,
      title: habit.title,
      kind: timed ? 'timed' : 'check',
      done,
      label: timed
        ? `${habit.title} · ${done ? habit.targetMinutes : Math.min(minutes, habit.targetMinutes!)}/${habit.targetMinutes}m`
        : habit.title,
    })
  }
  return chips.sort((a, b) => Number(a.done) - Number(b.done))
}