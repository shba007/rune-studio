import type { FlightRun } from '@/config/flight'

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

export function toneOf(outcome: string | null): 'ok' | 'bad' | 'stopped' | 'open' {
  if (outcome === 'succeeded') return 'ok'
  if (outcome === 'failed') return 'bad'
  if (outcome === null) return 'open'
  return 'stopped'
}