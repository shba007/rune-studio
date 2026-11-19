export interface FocusBlock {
  id: number
  title: string
  doneWhen: string
  plannedMs: number
  startedMs: number
  endsMs: number
  finishedMs: number | null
  outcome: 'done' | 'stopped' | null
  taskId: string | null
}

/** A fixed time the user must be somewhere. */
export interface FocusCommitment {
  id: number
  title: string
  startMs: number
  endMs: number
}

export interface FocusSnapshot {
  state: 'idle' | 'focusing'
  block: FocusBlock | null
  recent: FocusBlock[]
  /** Commitments that have not ended yet, soonest first. */
  commitments: FocusCommitment[]
}

export interface FocusProposal {
  title: string
  doneWhen: string
  minutes: number
}

export interface FocusSuggestion {
  source: 'model' | 'fallback'
  note: string | null
  /** The suggested block first, then a shorter and a longer one. */
  options: FocusProposal[]
}

/** Names an option by how it compares with the suggested one (the first). */
export function variantLabel(options: FocusProposal[], index: number): string {
  const first = options[0]
  const option = options[index]
  if (!first || !option || index === 0) return 'Suggested'
  if (option.minutes < first.minutes) return 'Shorter'
  if (option.minutes > first.minutes) return 'Longer'
  return 'Alternative'
}

export const EMPTY_FOCUS: FocusSnapshot = {
  state: 'idle',
  block: null,
  recent: [],
  commitments: [],
}

export const DEFAULT_MINUTES = 25
export const MIN_MINUTES = 1
export const MAX_MINUTES = 240
export const STEP_MINUTES = 5

export const remainingMs = (block: FocusBlock, now: number) => Math.max(0, block.endsMs - now)

export const isOver = (block: FocusBlock, now: number) => remainingMs(block, now) === 0

/** `m:ss`, or `h:mm:ss` from an hour up. Partial seconds count as a whole one. */
export function clock(ms: number): string {
  const total = Math.ceil(Math.max(0, ms) / 1000)
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = total % 60
  const two = (n: number) => String(n).padStart(2, '0')
  return hours ? `${hours}:${two(minutes)}:${two(seconds)}` : `${minutes}:${two(seconds)}`
}

/** Whole minutes left, rounded up, for the small badge on the orb. */
export const badgeMinutes = (ms: number) => Math.ceil(Math.max(0, ms) / 60_000)

export function stepMinutes(minutes: number, direction: 1 | -1): number {
  return Math.min(MAX_MINUTES, Math.max(MIN_MINUTES, minutes + direction * STEP_MINUTES))
}

export function loggedLine(block: FocusBlock): string {
  const spent = Math.max(
    1,
    Math.round(((block.finishedMs ?? block.endsMs) - block.startedMs) / 60_000),
  )
  return `${block.outcome === 'done' ? 'Done' : 'Stopped'} · ${spent} min · ${block.title}`
}
// These mirror src-tauri/src/focus_time.rs so the card can count down without
// asking the backend every second.
const MINUTE = 60_000
export const BUFFER_MS = 5 * MINUTE
export const WRAP_UP_MS = 5 * MINUTE

export const nextCommitment = (commitments: FocusCommitment[], now: number) =>
  commitments.find((c) => c.endMs > now)

/** Free time before the next commitment, buffer included. `null` is unlimited. */
export function freeMs(commitments: FocusCommitment[], now: number): number | null {
  const next = nextCommitment(commitments, now)
  return next ? Math.max(0, next.startMs - BUFFER_MS - now) : null
}

/** The commitment a running block should wrap up for. */
export function wrapUpFor(commitments: FocusCommitment[], now: number) {
  const next = nextCommitment(commitments, now)
  return next && next.startMs <= now + WRAP_UP_MS ? next : undefined
}

/** `52m`, `1h 5m` or `2h`. Rounds down: free time is never overstated. */
export function durationLabel(ms: number): string {
  const total = Math.floor(Math.max(0, ms) / MINUTE)
  const hours = Math.floor(total / 60)
  const minutes = total % 60
  if (!hours) return `${minutes}m`
  return minutes ? `${hours}h ${minutes}m` : `${hours}h`
}

/**
 * The next time the clock reads `time` ("14:30"), today or else tomorrow.
 * `null` for anything that is not a valid 24-hour time.
 */
export function startAt(time: string, now: Date): number | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(time.trim())
  if (!match) return null
  const [hours, minutes] = [Number(match[1]), Number(match[2])]
  if (hours > 23 || minutes > 59) return null
  const start = new Date(now)
  start.setHours(hours, minutes, 0, 0)
  if (start.getTime() <= now.getTime()) start.setDate(start.getDate() + 1)
  return start.getTime()
}

export const clockTime = (ms: number) =>
  new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })