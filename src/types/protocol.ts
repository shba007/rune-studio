export type AgentState =
  | 'idle'
  | 'listening'
  | 'planning'
  | 'awaiting_confirm'
  | 'acting'
  | 'observing'
  | 'reflecting'
  | 'loading_model'
  | 'done'
  | 'failed'
  | 'stopped'

export type TriState = 'yes' | 'no' | 'unknown'
export type Locality = 'local' | 'cloud' | 'unknown'

export interface ModelCapabilities {
  id: string
  modalitiesIn: string[]
  modalitiesOut: string[]
  audioIn: TriState
  imageIn: TriState
  speechOut: TriState
  toolCalls: TriState
  thinkingControl: TriState
  maxImageEdgePx: number | null
  contextTokens: number | null
  locality: Locality
  resident: TriState
  vramMbEstimate: number | null
  loadMsEstimate: number | null
  status: string | null
  rawCapabilities: string[]
}

export interface ModelRoles {
  agent: string
  verifier: string
  analysis: string
  voice: string
  decision: string
  advisor: string
}

export type AnalysisGroup = 'entities' | 'regions' | 'structure' | 'depth'
export type VisualAnalysisVisibility = Record<AnalysisGroup, boolean>

export interface Settings {
  visualAnalysis?: VisualAnalysisVisibility
  verifyMode: 'off' | 'background' | 'blocking'
  demoMode: boolean
  recordSessionVideo: 'off' | 'motion'
  modelImageResolution: '720' | '1280' | '1920' | '2560' | 'native'
  recordingResolution: '720' | '1280' | '1920' | '2560' | 'native'
  offerReview: 'always' | 'failures' | 'never'
  reflectionEnabled: boolean
  idleMinutes: number
  archiveKeepDays: number
  baseUrl: string
  model: string
  models: ModelRoles
  maxSteps: number
  toggleHotkey: string
  stopHotkey: string
  externalAgent: boolean
  externalAgentSince: number
  showThinking: boolean
  askEveryAction: boolean
  developerMode: boolean
  fastChecks: boolean
  decisionModel: string
  decisionUrl: string
  skipSwapForOptionalRoles?: boolean
  firstCallIdleAllowanceSecs?: number
  lifeEnabled?: boolean
  lifeConsents?: Record<string, boolean>
  retentionDays?: Record<string, number>
}

export interface BackendStatus {
  reachable: boolean
  authOk: boolean
  missingModels: string[]
  modelCapabilities: string[]
  latencyMs: number | null
  error: string | null
  models?: ModelCapabilities[]
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

export interface CaptureReport {
  monitor: MonitorInfo
  width: number
  height: number
  bytes: number
  millis: number
  thumbnail: string
}

export interface ClickReport {
  monitor: MonitorInfo
  pixelX: number
  pixelY: number
  cursorX: number
  cursorY: number
}

export type PillMode = 'ambient' | 'open' | 'task' | 'analysis'

export const STATE_CHANGED = 'state_changed'
export const SUMMON = 'summon'
export const AUDIO_LEVEL = 'audio_level'
export const VOICE_HEARD = 'voice_heard'

export interface VoiceHeardPayload {
  text: string
  note: string
}
export const STEP_STARTED = 'step_started'
export const TASK_DONE = 'task_done'
export const TASK_VERIFIED = 'task_verified'
export const EXTERNAL_AGENT = 'external_agent'

export interface ExternalAgentPayload {
  agent: string | null
  id: string | null
  known: boolean
  verified: boolean
  declared: string | null
}
export const OBSERVE_STARTED = 'observe_started'
export const TYPED = 'typed'
export const PLAN_UPDATED = 'plan_updated'
export const CONFIRM_REQUEST = 'confirm_request'
export const CONFIRM_RESOLVED = 'confirm_resolved'

export interface ConfirmRequestPayload {
  id: string
  source: string
  text: string
  reason: string
  risk: 'low' | 'medium' | 'high'
  reasons: string[]
  timeoutSecs: number
  kind: 'approval' | 'question' | 'restart'
}
export const THINKING = 'thinking'

export interface PlanUpdatedPayload {
  goals: string[]
  current: number
  finished: boolean
}

export interface ThinkingPayload {
  text: string
  done: boolean
}

export interface PillRect {
  x: number
  y: number
  width: number
  height: number
}

export interface TypedPayload {
  point: { x: number; y: number } | null
}

export interface ObserveStartedPayload {
  region: [number, number, number, number] | null
  continuous?: boolean
}

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
  detail: string
  point: { x: number; y: number } | null
  double: boolean
  button: 'left' | 'right' | 'middle'
  durationMs: number
}

export interface TaskDonePayload {
  summary: string
  success: boolean
}

export interface TaskVerifiedPayload {
  runId: string
  result: 'verified' | 'contradicted' | 'unsure'
  evidence: string
}

export const ANALYSIS_UPDATED = 'analysis_updated'

export type AnalysisStatus = 'idle' | 'capturing' | 'asking' | 'done' | 'error' | 'cancelled'
export type FindingKind = 'text' | 'control' | 'image' | 'region' | 'issue' | 'other'

export type Box = [number, number, number, number]

export interface Finding {
  pose?: Pose | null
  group?: AnalysisGroup
  n: number
  label: string
  kind: FindingKind
  box: Box | null
  description: string
  evidence: string
  point: [number, number] | null
}

export interface Pose {
  rejected?: number
  keypoints: { label: string; x: number; y: number }[]
  connections: [number, number][]
}

export interface AnalysisSnapshot {
  visibleGroups?: VisualAnalysisVisibility
  screenshot?: string | null
  status: AnalysisStatus
  id: string
  question: string
  answer: string
  findings: Finding[]
  model: string
  latencyMs: number
  error: string | null
  createdUtc: string
  monitor: MonitorInfo | null
  stale: boolean
  boxesRejected: number
  selected: number | null
}

export const OVERLAY_MODE = 'overlay_mode'
export type OverlayOwner = 'none' | 'control' | 'analysis'

export interface CaptureFeedback {
  serial: number
  monitor: MonitorInfo
  remainingMs: number
}

export interface ScreenRecording {
  id: number
  phase: 'idle' | 'starting' | 'recording' | 'saving' | 'done' | 'cancelled' | 'failed'
  seconds: number
  monitor: MonitorInfo | null
  path: string | null
  error: string | null
}

export interface OverlayInfo {
  x: number
  y: number
  mode: OverlayOwner
  primary: boolean
  width: number
  height: number
  scaleFactor: number
  captureFeedback?: CaptureFeedback | null
  recording?: ScreenRecording
}
export const REFLECTION_PROMPT = 'reflection_prompt'
export const REFLECTION_RESOLVED = 'reflection_resolved'
export const REFLECTION_PROGRESS = 'reflection_progress'

export interface ReflectionPromptPayload {
  timeoutSecs: number
  endsInMs: number
}

export interface ReflectionProgressPayload {
  stage: string
  done: number
  total: number
}
export interface ScreenRequestSnapshot {
  id: number | null
  status: 'idle' | 'queued' | 'delivered' | 'captured' | 'cancelled' | 'expired'
  waiting: boolean
}