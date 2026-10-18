// Mirrors the Rust side (events.rs, settings.rs, backend.rs). Keep in sync by hand.

export type AgentState =
  | 'idle'
  | 'listening'
  | 'planning'
  | 'awaiting_confirm'
  | 'acting'
  | 'observing'
  | 'done'
  | 'failed'
  | 'stopped'

export interface Settings {
  baseUrl: string
  visionModel: string
  asrModel: string
  maxSteps: number
  toggleHotkey: string
  stopHotkey: string
}

export interface BackendStatus {
  reachable: boolean
  authOk: boolean
  missingModels: string[]
  latencyMs: number | null
  error: string | null
}

export type PillMode = 'ambient' | 'open' | 'settings'

export const STATE_CHANGED = 'state_changed'
export const SUMMON = 'summon'
export const OPEN_SETTINGS = 'open_settings'
export const AUDIO_LEVEL = 'audio_level'

export interface StateChangedPayload {
  state: AgentState
}