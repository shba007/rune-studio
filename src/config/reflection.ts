/** Whole seconds left on the prompt's countdown, never negative. `endsAt` and `now` are in ms on one clock. */
export function secondsLeft(now: number, endsAt: number): number {
  return Math.max(0, Math.ceil((endsAt - now) / 1000))
}

/** `m:ss` for a number of seconds. */
export function clock(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}