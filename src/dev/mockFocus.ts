import { emit } from '@tauri-apps/api/event'

import type { Habit } from '@/config/habits'
import type { FocusBlock, FocusCommitment, FocusSnapshot } from '@/config/focus'

// Browser mock of the Focus commands, mirroring src-tauri/src/focus.rs closely
// enough to exercise the UI. Nothing is saved: a reload starts empty.
const MINUTE = 60_000
const BUFFER = 5 * MINUTE

let nextId = 1
let running: FocusBlock | null = null
const finished: FocusBlock[] = []
let commitments: FocusCommitment[] = []

// Two sample habits: a plain check-in and a timed one with a 30 minute target.
const habits: Habit[] = [
  {
    id: 'habit-water',
    title: 'Drink water',
    targetMinutes: null,
    state: 'pending',
    dueMs: Date.now() - MINUTE,
    completedMs: null,
    creditedMs: 0,
  },
  {
    id: 'habit-read',
    title: 'Read',
    targetMinutes: 30,
    state: 'pending',
    dueMs: Date.now() - MINUTE,
    completedMs: null,
    creditedMs: 0,
  },
]
const credited = (id: string) =>
  finished
    .filter((b) => b.taskId === id)
    .reduce((sum, b) => sum + Math.min((b.finishedMs ?? 0) - b.startedMs, b.plannedMs), 0)

const snapshot = (): FocusSnapshot => ({
  state: running ? 'focusing' : 'idle',
  block: running,
  recent: [...finished].reverse().slice(0, 5),
  commitments: commitments
    .filter((c) => c.endMs > Date.now())
    .sort((a, b) => a.startMs - b.startMs),
})
const announce = async () => {
  const next = snapshot()
  await emit('focus_state_changed', next)
  return next
}
const free = () => {
  const next = snapshot().commitments[0]
  return next ? Math.max(0, next.startMs - BUFFER - Date.now()) : null
}

export async function mockFocus(command: string, args: Record<string, unknown>): Promise<unknown> {
  const text = (value: unknown) => (typeof value === 'string' ? value : '')
  const now = Date.now()
  switch (command) {
    case 'focus_snapshot':
      return snapshot()
    case 'focus_start': {
      const title = text(args.title).trim()
      const minutes = Number(args.minutes)
      if (!title) throw 'Give the focus block a title.'
      if (!(minutes >= 1 && minutes <= 240)) throw 'A focus block is 1 to 240 minutes long.'
      if (running) throw 'A focus block is already running. Finish it first.'
      running = {
        id: nextId++,
        title,
        doneWhen: text(args.doneWhen).trim(),
        plannedMs: minutes * MINUTE,
        startedMs: now,
        endsMs: now + minutes * MINUTE,
        finishedMs: null,
        outcome: null,
        taskId: null,
      }
      return announce()
    }
    case 'focus_start_task': {
      const habit = habits.find((h) => h.id === args.taskId)
      if (!habit) throw 'This task is not waiting on you any more.'
      if (running) throw 'A focus block is already running. Finish it first.'
      const left = Math.max(
        5,
        (habit.targetMinutes ?? 25) - Math.floor(credited(habit.id) / MINUTE),
      )
      const minutes = Math.min(25, left)
      running = {
        id: nextId++,
        title: habit.title,
        doneWhen: '',
        plannedMs: minutes * MINUTE,
        startedMs: now,
        endsMs: now + minutes * MINUTE,
        finishedMs: null,
        outcome: null,
        taskId: habit.id,
      }
      return announce()
    }
    case 'focus_finish': {
      if (!running) throw 'No focus block is running.'
      const block: FocusBlock = {
        ...running,
        finishedMs: now,
        outcome: args.outcome as 'done' | 'stopped',
      }
      finished.push(block)
      running = null
      // A timed habit whose time today reaches its target is checked in.
      const habit = habits.find((h) => h.id === block.taskId)
      if (habit?.targetMinutes && credited(habit.id) >= habit.targetMinutes * MINUTE) {
        habit.state = 'completed'
        habit.completedMs = now
      }
      await emit('workspace_tasks_changed')
      return announce()
    }
    case 'focus_extend': {
      if (!running) throw 'No focus block is running.'
      const end = Math.max(running.endsMs, now) + 5 * MINUTE
      const next = snapshot().commitments[0]
      if (next && end > Math.max(next.startMs - BUFFER, now))
        throw `No room to extend: "${next.title}" starts in ${Math.ceil((next.startMs - now) / MINUTE)} min.`
      running = { ...running, plannedMs: running.plannedMs + (end - running.endsMs), endsMs: end }
      return announce()
    }
    case 'focus_commitment_add': {
      const minutes = Number(args.minutes)
      const startMs = Number(args.startMs)
      if (!text(args.title).trim()) throw 'Give the commitment a title of up to 120 characters.'
      if (startMs + minutes * MINUTE <= now) throw 'That commitment is already over.'
      commitments.push({
        id: nextId++,
        title: text(args.title).trim(),
        startMs,
        endMs: startMs + minutes * MINUTE,
      })
      return announce()
    }
    case 'focus_commitment_remove':
      commitments = commitments.filter((c) => c.id !== args.id)
      return announce()
    case 'focus_suggest': {
      const goal = text(args.goal).trim()
      if (!goal) throw 'Type what you want to work on first.'
      await new Promise((resolve) => setTimeout(resolve, 600))
      const room = free()
      if (room !== null && room < MINUTE)
        throw 'You have no free time before your next commitment. Remove it, or start a block yourself.'
      const fit = (m: number) =>
        room === null ? m : Math.max(1, Math.min(m, Math.floor(room / MINUTE)))
      const options = [
        {
          title: `Make progress on: ${goal}`.slice(0, 80),
          doneWhen: 'First draft is saved',
          minutes: fit(45),
        },
        { title: 'Outline the first step', doneWhen: 'Outline written down', minutes: fit(20) },
        {
          title: `Finish and review: ${goal}`.slice(0, 80),
          doneWhen: 'Reviewed and ready',
          minutes: fit(75),
        },
      ].filter(
        (o, i, all) => all.findIndex((x) => x.title === o.title && x.minutes === o.minutes) === i,
      )
      return { source: 'model', note: null, options }
    }
    case 'habits_state':
      return habits.map((h) => ({ ...h, creditedMs: credited(h.id) }))
    case 'habit_check_in': {
      const habit = habits.find((h) => h.id === args.id)
      if (!habit || habit.state === 'completed') throw 'This habit is already checked in for today.'
      habit.state = 'completed'
      habit.completedMs = now
      await emit('workspace_tasks_changed')
      return null
    }
    default:
      return undefined
  }
}

export const isFocusCommand = (command: string) => command.startsWith('focus_')
export const isHabitCommand = (command: string) => command.startsWith('habit')