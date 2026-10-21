export function secondsLeft(now: number, endsAt: number): number {
  return Math.max(0, Math.ceil((endsAt - now) / 1000))
}

export function clock(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}