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
  handedOffMs: number | null
}

export interface FocusCommitment {
  id: number
  title: string
  startMs: number
  endMs: number
  source: 'manual' | 'calendar'
}

export interface CalendarReport {
  state: 'synced' | 'unavailable'
  events: number
  message: string
}

export interface FocusPreset {
  id: string
  label: string
  summary: string
  defaultMinutes: number
  capMinutes: number
  bufferMinutes: number
  wrapUpMinutes: number
  extendMinutes: number
  doneWhenLabel: string
}

export const DEFAULT_PRESET: FocusPreset = {
  id: 'engineer',
  label: 'Engineer',
  summary: 'Long, deep blocks; a short buffer.',
  defaultMinutes: 25,
  capMinutes: 120,
  bufferMinutes: 5,
  wrapUpMinutes: 5,
  extendMinutes: 5,
  doneWhenLabel: 'Done when',
}

export interface FocusSnapshot {
  state: 'idle' | 'focusing'
  block: FocusBlock | null
  recent: FocusBlock[]
  commitments: FocusCommitment[]
  preset: FocusPreset
  presets: FocusPreset[]
}

export interface FocusProposal {
  title: string
  doneWhen: string
  minutes: number
}

export interface FocusSuggestion {
  source: 'model' | 'fallback'
  note: string | null
  options: FocusProposal[]
}

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
  preset: DEFAULT_PRESET,
  presets: [DEFAULT_PRESET],
}

export const DEFAULT_MINUTES = 25
export const MIN_MINUTES = 1
export const MAX_MINUTES = 240
export const STEP_MINUTES = 5

export const remainingMs = (block: FocusBlock, now: number) => Math.max(0, block.endsMs - now)

export const isOver = (block: FocusBlock, now: number) => remainingMs(block, now) === 0

export function clock(ms: number): string {
  const total = Math.ceil(Math.max(0, ms) / 1000)
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = total % 60
  const two = (n: number) => String(n).padStart(2, '0')
  return hours ? `${hours}:${two(minutes)}:${two(seconds)}` : `${minutes}:${two(seconds)}`
}

export const badgeMinutes = (ms: number) => Math.ceil(Math.max(0, ms) / 60_000)

export function stepMinutes(minutes: number, direction: 1 | -1, cap: number = MAX_MINUTES): number {
  return Math.min(cap, Math.max(MIN_MINUTES, minutes + direction * STEP_MINUTES))
}

export function loggedLine(block: FocusBlock): string {
  const spent = Math.max(
    1,
    Math.round(((block.finishedMs ?? block.endsMs) - block.startedMs) / 60_000),
  )
  return `${block.outcome === 'done' ? 'Done' : 'Stopped'} · ${spent} min · ${block.title}`
}
const MINUTE = 60_000

export const nextCommitment = (commitments: FocusCommitment[], now: number) =>
  commitments.find((c) => c.endMs > now)

export function freeMs(
  commitments: FocusCommitment[],
  now: number,
  preset: FocusPreset = DEFAULT_PRESET,
): number | null {
  const next = nextCommitment(commitments, now)
  return next ? Math.max(0, next.startMs - preset.bufferMinutes * MINUTE - now) : null
}

export function wrapUpFor(
  commitments: FocusCommitment[],
  now: number,
  preset: FocusPreset = DEFAULT_PRESET,
) {
  const next = nextCommitment(commitments, now)
  return next && next.startMs <= now + preset.wrapUpMinutes * MINUTE ? next : undefined
}

export function durationLabel(ms: number): string {
  const total = Math.floor(Math.max(0, ms) / MINUTE)
  const hours = Math.floor(total / 60)
  const minutes = total % 60
  if (!hours) return `${minutes}m`
  return minutes ? `${hours}h ${minutes}m` : `${hours}h`
}

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