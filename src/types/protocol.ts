// Mirrors the Rust side (events.rs, settings.rs, backend.rs). Keep in sync by hand.

export type AgentState =
  | 'idle'
  | 'listening'
  | 'planning'
  | 'awaiting_confirm'
  | 'acting'
  | 'observing'
  | 'reflecting'
  | 'done'
  | 'failed'
  | 'stopped'

export interface Settings {
  /** What happens with the independent check of a finished task. No UI yet: edit settings.json. */
  verifyMode: 'off' | 'background' | 'blocking'
  /** Let screen recorders capture Rune's windows; the model still never sees them. */
  demoMode: boolean
  /** `off`: a storyboard of the step screenshots; `motion`: also the screen twice a second (unmeasured cost). */
  recordSessionVideo: 'off' | 'motion'
  /** When the pill offers to review a finished session. */
  offerReview: 'always' | 'failures' | 'never'
  /** Offer an idle review of recent tasks. Off until the job does something real. */
  reflectionEnabled: boolean
  /** Minutes with no activity before the review prompt (1 to 120). */
  idleMinutes: number
  /** Delete session folders older than this many days at startup; 0 keeps everything. No UI yet. */
  archiveKeepDays: number
  baseUrl: string
  model: string
  maxSteps: number
  toggleHotkey: string
  stopHotkey: string
  externalAgent: boolean
  /** Unix seconds of the last `rune-studio-cli enable`. The UI passes it back unchanged. */
  externalAgentSince: number
  /** Let the model think, and show that thinking. Slower, so off by default. */
  showThinking: boolean
  /** Ask before every action, not only the risky ones. */
  askEveryAction: boolean
  /** Use the fast decision model for quick checks (skip planning, spot a finished task, risk). */
  fastChecks: boolean
  decisionModel: string
  decisionUrl: string
}

export interface BackendStatus {
  reachable: boolean
  authOk: boolean
  missingModels: string[]
  modelCapabilities: string[]
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

export type PillMode = 'ambient' | 'open' | 'task' | 'analysis' | 'settings'

export const STATE_CHANGED = 'state_changed'
export const SUMMON = 'summon'
export const OPEN_SETTINGS = 'open_settings'
export const AUDIO_LEVEL = 'audio_level'
export const VOICE_HEARD = 'voice_heard'

/** Something was said: the words, and what the app did about them. */
export interface VoiceHeardPayload {
  text: string
  note: string
}
export const STEP_STARTED = 'step_started'
export const TASK_DONE = 'task_done'
export const TASK_VERIFIED = 'task_verified'
export const EXTERNAL_AGENT = 'external_agent'

/** Who is in control from outside (events.rs `ExternalAgent`); all null/false when nobody is. */
export interface ExternalAgentPayload {
  /** The display name ("Claude", or an unknown agent's own name). */
  agent: string | null
  /** The brand id of `src/config/agents.ts`. */
  id: string | null
  known: boolean
  /** The agent's registration named it, not only the agent itself. */
  verified: boolean
  /** What the agent called itself, for a tooltip. */
  declared: string | null
}
export const OBSERVE_STARTED = 'observe_started'
export const TYPED = 'typed'
export const PLAN_UPDATED = 'plan_updated'
export const CONFIRM_REQUEST = 'confirm_request'
export const CONFIRM_RESOLVED = 'confirm_resolved'

/** An action is waiting for the user's yes or no. */
export interface ConfirmRequestPayload {
  id: string
  /** Who wants to do it: Rune, or an outside agent's name. */
  source: string
  /** What it wants to do, in a short line. */
  text: string
  /** Why, in the agent's words. */
  reason: string
  risk: 'low' | 'medium' | 'high'
  /** What made it risky, in plain words. */
  reasons: string[]
  timeoutSecs: number
  /** An action that wants a yes, or a question the agent asks. */
  kind: 'approval' | 'question'
}
export const THINKING = 'thinking'

/** The plan for the task: its goals, and which one is being worked on. */
export interface PlanUpdatedPayload {
  goals: string[]
  /** Index of the goal being worked on; equal to the number of goals when all are done. */
  current: number
  finished: boolean
}

/** What the model is thinking, as it streams in (only with Show thinking on). */
export interface ThinkingPayload {
  text: string
  done: boolean
}

/** Where the pill is on the desktop, in physical pixels. */
export interface PillRect {
  x: number
  y: number
  width: number
  height: number
}

/** Text was just typed. `point` is the caret (or the mouse), in physical desktop pixels. */
export interface TypedPayload {
  point: { x: number; y: number } | null
}

/** The agent is taking a screenshot. `region` is the zoomed part (0 to 1000 units), or null for the whole screen. */
export interface ObserveStartedPayload {
  region: [number, number, number, number] | null
  /** A recording: the wave keeps going until `observe_stopped`. A screenshot is one wave. */
  continuous?: boolean
}

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

/** The independent check of a finished task (`verifyMode` background). */
export interface TaskVerifiedPayload {
  runId: string
  result: 'verified' | 'contradicted' | 'unsure'
  evidence: string
}

export const ANALYSIS_UPDATED = 'analysis_updated'

export type AnalysisStatus = 'idle' | 'capturing' | 'asking' | 'done' | 'error' | 'cancelled'
export type FindingKind = 'text' | 'control' | 'image' | 'region' | 'issue' | 'other'

/** `[x0, y0, x1, y1]`, each 0 to 1000 across the whole screenshot. */
export type Box = [number, number, number, number]

/** One thing the model points at on the screenshot (analysis/report.rs `Finding`). */
export interface Finding {
  n: number
  label: string
  kind: FindingKind
  /** Null when the model could not locate it: the finding is listed but not drawn. */
  box: Box | null
  description: string
  evidence: string
  /** `[x, y]` in 0..1000; only with the box + point request variant, which is off. */
  point: [number, number] | null
}

/** The screen analysis as the cards and the screen markers show it (analysis/state.rs `Snapshot`). */
export interface AnalysisSnapshot {
  status: AnalysisStatus
  id: string
  question: string
  answer: string
  findings: Finding[]
  model: string
  latencyMs: number
  error: string | null
  createdUtc: string
  /** The monitor that was captured: box coordinates are relative to it. Null before a capture. */
  monitor: MonitorInfo | null
  /** The screen changed after the capture: no boxes are drawn, the findings text stays. */
  stale: boolean
  /** How many boxes the quality guard dropped. */
  boxesRejected: number
  /** The finding picked in the cards, by number. */
  selected: number | null
}

export const OVERLAY_MODE = 'overlay_mode'
export type OverlayOwner = 'none' | 'control' | 'analysis'

/** What an overlay page knows about itself (overlay.rs `OverlayInfo`). */
export interface OverlayInfo {
  mode: OverlayOwner
  /** This window is on the primary monitor, where the analysis draws. */
  primary: boolean
  /** The window's size in physical pixels. */
  width: number
  height: number
  scaleFactor: number
}
export const REFLECTION_PROMPT = 'reflection_prompt'
export const REFLECTION_RESOLVED = 'reflection_resolved'
export const REFLECTION_PROGRESS = 'reflection_progress'

/** The idle review is offered; the window counts down on its own clock from `endsInMs`. */
export interface ReflectionPromptPayload {
  timeoutSecs: number
  endsInMs: number
}

/** How far the review is; `total` 0 means it ended. */
export interface ReflectionProgressPayload {
  stage: string
  done: number
  total: number
}