export type OrbAction = 'busy' | 'stop' | 'open' | 'close'

export interface OrbHint {
  action: OrbAction
  label: string
}

export function orbHint(state: { updating: boolean; working: boolean; ambient: boolean }): OrbHint {
  if (state.updating) return { action: 'busy', label: 'Updating' }
  if (state.working) return { action: 'stop', label: 'Stop' }
  if (state.ambient) return { action: 'open', label: 'Open' }
  return { action: 'close', label: 'Close' }
}