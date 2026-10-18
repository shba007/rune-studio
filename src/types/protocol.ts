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
  externalAgent: boolean
  /** Unix seconds of the last `rune-studio enable`. The UI passes it back unchanged. */
  externalAgentSince: number
}

export interface BackendStatus {
  reachable: boolean
  authOk: boolean
  missingModels: string[]
  latencyMs: number | null
  error: string | null
}

export interface MonitorInfo {
  id: number
  x: number
  y: number
  width: number
  height: number
  scaleFactor: number
  primary: boolean
}

/** Result of the dev-only screen capture: what the model would see. */
export interface CaptureReport {
  monitor: MonitorInfo
  width: number
  height: number
  bytes: number
  millis: number
  thumbnail: string
}

/** Result of the dev-only click: where it was aimed and where the cursor really ended up. */
export interface ClickReport {
  monitor: MonitorInfo
  pixelX: number
  pixelY: number
  cursorX: number
  cursorY: number
}

export type PillMode = 'ambient' | 'open' | 'settings'

export const STATE_CHANGED = 'state_changed'
export const SUMMON = 'summon'
export const OPEN_SETTINGS = 'open_settings'
export const AUDIO_LEVEL = 'audio_level'
export const STEP_STARTED = 'step_started'
export const TASK_DONE = 'task_done'
export const EXTERNAL_AGENT = 'external_agent'

/** What Settings shows for the External agent group. `available` is false in normal builds. */
export interface BridgeInfo {
  available: boolean
  url: string
  command: string
}

export interface StateChangedPayload {
  state: AgentState
}
export interface StepStartedPayload {
  step: number
  tool: string
  reason: string
  text: string
  /** Typed text, keys, app name or wait time, for the on-screen caption. */
  detail: string
  /** Where a click lands, in physical desktop pixels. */
  point: { x: number; y: number } | null
  double: boolean
  button: 'left' | 'right' | 'middle'
  /** How long the action takes: the cursor glide for a click, the typing time for text. */
  durationMs: number
}

export interface TaskDonePayload {
  summary: string
  success: boolean
}