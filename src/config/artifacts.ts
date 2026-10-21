import type { FlightRun } from '@/config/flight'

/** "Today 3:29 PM", "Yesterday 9:05 AM", "20 Oct 3:29 PM": when a session ran, on the user's own clock. */
export function whenText(startedUtc: string | null, now: Date = new Date()): string {
  const at = startedUtc ? new Date(startedUtc) : null
  if (!at || Number.isNaN(at.getTime())) return ''
  const time = at.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
  const days = Math.round(
    (new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() -
      new Date(at.getFullYear(), at.getMonth(), at.getDate()).getTime()) /
      86_400_000,
  )
  if (days === 0) return `Today ${time}`
  if (days === 1) return `Yesterday ${time}`
  return `${at.toLocaleDateString([], { day: 'numeric', month: 'short' })} ${time}`
}

/** How long a session took: "45s", "2m 5s". Empty while it is still open. */
export function durationText(totalSeconds: number): string {
  const seconds = Math.max(0, Math.round(totalSeconds))
  if (seconds < 60) return `${seconds} s`
  const minutes = Math.floor(seconds / 60)
  const rest = seconds % 60
  if (minutes < 60) return rest === 0 ? `${minutes} min` : `${minutes} min ${rest} s`
  const hours = Math.floor(minutes / 60)
  return minutes % 60 === 0 ? `${hours} h` : `${hours} h ${minutes % 60} min`
}

export function lengthText(run: Pick<FlightRun, 'startedUtc' | 'endedUtc'>): string {
  const start = run.startedUtc ? Date.parse(run.startedUtc) : Number.NaN
  const end = run.endedUtc ? Date.parse(run.endedUtc) : Number.NaN
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) return ''
  return durationText((end - start) / 1000)
}

/** One dot per step, at most `max`; `more` says there were further steps. */
export function stepDots(steps: number, max = 8): { count: number; more: boolean } {
  return { count: Math.min(Math.max(steps, 0), max), more: steps > max }
}

/** How a session ended, as the tone of its dot. */
export function toneOf(outcome: string | null): 'ok' | 'bad' | 'stopped' | 'open' {
  if (outcome === 'succeeded') return 'ok'
  if (outcome === 'failed') return 'bad'
  if (outcome === null) return 'open'
  return 'stopped'
}