/**
 * The Flight Player's rules, kept out of the components so they can be tested on their own: how the
 * video's clock maps to steps and back, what the action layer draws, what is hidden by default, which
 * drawer tabs have data, and which rows of a long list are on screen.
 */

export interface FlightRun {
  id: string
  task: string
  source: string
  outcome: string | null
  startedUtc: string | null
  endedUtc: string | null
  steps: number
  verdict: string | null
}

export interface FlightVideo {
  kind?: 'storyboard' | 'motion'
  file?: string
  unavailable?: string
  [key: string]: unknown
}

export interface FlightHeader {
  id: string
  task: string
  source: string
  model: string | null
  startedUtc: string
  endedUtc: string | null
  outcome: string | null
  summary: string | null
  plan: unknown
  steps: number
  retries: number
  durationSecs: number | null
  verdict: { result: string; evidence: string | null; mode: string; error: string | null } | null
  video: FlightVideo | null
  sessionDir: string | null
}

export interface FlightPhases {
  observe: number | null
  screenCheck: number | null
  finishCheck: number | null
  decide: number | null
  gate: number | null
  act: number | null
  settle: number | null
}

export interface FlightStep {
  step: number
  startedUtc: string | null
  goal: number | null
  tool: string | null
  argsJson: string | null
  reason: string | null
  riskStated: string | null
  riskFinal: string | null
  riskReasonsJson: string | null
  gate: string | null
  result: string | null
  error: string | null
  observationPath: string | null
  observationFile: string | null
  promptText: string | null
  thinkingChars: number | null
  completionTokens: number | null
  phases: FlightPhases
}

export interface FlightEvent {
  id: number
  atUtc: string | null
  kind: string | null
  dataJson: string | null
}

export interface FlightCall {
  id: number
  step: number | null
  kind: string
  attempt: number
  startedUtc: string
  durationMs: number
  waitBeforeMs: number
  status: number | null
  outcome: string
  error: string | null
  truncated: boolean
  model: string
  promptTokens: number | null
  completionTokens: number | null
}

// ------------------------------------------------------------------------------- the clock

/** Seconds-resolution ISO time to milliseconds, or null when it cannot be read. */
export function parseIso(text: string | null | undefined): number | null {
  if (!text) return null
  const value = Date.parse(text)
  return Number.isFinite(value) ? value : null
}

/**
 * When each step began, in milliseconds on the session's clock (the video's clock). Step times have
 * one-second resolution, so steps in the same second share a start; the starts never go backwards, and
 * the first step is at 0 (the video starts with its first frame, whenever that frame was taken).
 */
export function stepStarts(runStartedUtc: string | null, steps: FlightStep[]): number[] {
  const absolute = absoluteStarts(runStartedUtc, steps)
  const shift = absolute[0] ?? 0
  return absolute.map((start) => start - shift)
}

/** The same starts measured from the run's own start (before the first step is moved to 0). */
function absoluteStarts(runStartedUtc: string | null, steps: FlightStep[]): number[] {
  const origin = parseIso(runStartedUtc)
  const starts: number[] = []
  let previous = 0
  for (const step of steps) {
    const at = parseIso(step.startedUtc)
    const raw = at !== null && origin !== null ? Math.max(0, at - origin) : previous
    const start = Math.max(previous, raw + Math.max(0, step.phases.observe ?? 0))
    starts.push(start)
    previous = start
  }
  return starts
}

/** The step on screen at `ms`: the last one that has begun. -1 when there are no steps. */
export function stepAtTime(starts: number[], ms: number): number {
  if (starts.length === 0) return -1
  let low = 0
  let high = starts.length - 1
  let found = 0
  while (low <= high) {
    const middle = (low + high) >> 1
    if ((starts[middle] ?? 0) <= ms) {
      found = middle
      low = middle + 1
    } else {
      high = middle - 1
    }
  }
  return found
}

/** The time to seek to for a step (a step out of range is clamped). */
export function timeOfStep(starts: number[], index: number): number {
  if (starts.length === 0) return 0
  return starts[Math.min(starts.length - 1, Math.max(0, index))] ?? 0
}

/** How long the session lasts: the video's length when there is one, else the end of the record. */
export function totalMs(starts: number[], header: Pick<FlightHeader, 'durationSecs'>): number {
  const last = starts.at(-1) ?? 0
  return Math.max((header.durationSecs ?? 0) * 1000, last + 1000)
}

export interface Segment {
  index: number
  start: number
  length: number
}

/** The timeline's segments: each step as wide as its real duration (the last one runs to the end). */
export function segments(starts: number[], total: number): Segment[] {
  return starts.map((start, index) => ({
    index,
    start,
    length: Math.max(0, (starts[index + 1] ?? total) - start),
  }))
}

export const PHASES = ['observe', 'decide', 'gate', 'act', 'settle'] as const
export type Phase = (typeof PHASES)[number]

/** A step's phase times in drawing order (screen and finish checks count toward observe). */
export function phaseTimes(phases: FlightPhases): { phase: Phase; ms: number }[] {
  const ms = (value: number | null) => Math.max(0, value ?? 0)
  return [
    { phase: 'observe', ms: ms(phases.observe) + ms(phases.screenCheck) + ms(phases.finishCheck) },
    { phase: 'decide', ms: ms(phases.decide) },
    { phase: 'gate', ms: ms(phases.gate) },
    { phase: 'act', ms: ms(phases.act) },
    { phase: 'settle', ms: ms(phases.settle) },
  ]
}

/** `0:55` or `1:02:03`. */
export function clock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = String(total % 60).padStart(2, '0')
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, '0')}:${seconds}`
    : `${minutes}:${seconds}`
}

// -------------------------------------------------------------------- what a step did

function argsOf(step: Pick<FlightStep, 'argsJson'>): Record<string, unknown> {
  if (!step.argsJson) return {}
  try {
    const value: unknown = JSON.parse(step.argsJson)
    return typeof value === 'object' && value !== null && !Array.isArray(value)
      ? (value as Record<string, unknown>)
      : {}
  } catch {
    return {}
  }
}

/** The tools whose arguments are what the user typed. */
export const TYPING_TOOLS = ['type_text'] as const

export function isTyping(step: Pick<FlightStep, 'tool'>): boolean {
  return TYPING_TOOLS.some((name) => name === step.tool)
}

/** What typed text is shown as until the viewer chooses SHOW. */
export function hiddenText(text: string): string {
  const count = Array.from(text).length
  return `•••• (${count} ${count === 1 ? 'character' : 'characters'})`
}

/** The arguments of a step as text, with typed text hidden unless `show`. */
export function argsText(step: FlightStep, show: boolean): string {
  if (!step.argsJson) return ''
  if (!isTyping(step) || show) return step.argsJson
  const args = argsOf(step)
  if (typeof args.text === 'string') return JSON.stringify({ ...args, text: hiddenText(args.text) })
  return JSON.stringify({ ...args, text: '••••' })
}

/** Any text that might repeat what was typed, hidden for a typing step. */
export function guardText(step: FlightStep, text: string | null, show: boolean): string {
  if (!text) return ''
  if (show || !isTyping(step)) return text
  const typed = argsOf(step).text
  return typeof typed === 'string' && typed.length > 0
    ? text.split(typed).join(hiddenText(typed))
    : text
}

export type ActionLayer =
  | { kind: 'click'; x: number; y: number; label: string }
  | { kind: 'scroll'; x: number; y: number; label: string }
  | { kind: 'type'; text: string }
  | { kind: 'keys'; text: string }
  | { kind: 'none' }

const BUTTONS: Record<string, string> = {
  left: 'CLICK',
  right: 'RIGHT-CLICK',
  middle: 'MIDDLE-CLICK',
}

/** What the light overlay draws over the picture for a step (coordinates are 0 to 1000). */
export function actionLayer(step: FlightStep | undefined, show: boolean): ActionLayer {
  if (!step || !step.tool) return { kind: 'none' }
  const args = argsOf(step)
  const number = (value: unknown) =>
    typeof value === 'number' && Number.isFinite(value) ? value : null
  const x = number(args.x)
  const y = number(args.y)
  switch (step.tool) {
    case 'click': {
      if (x === null || y === null) return { kind: 'none' }
      const button = typeof args.button === 'string' ? (BUTTONS[args.button] ?? 'CLICK') : 'CLICK'
      return { kind: 'click', x, y, label: args.double === true ? 'DOUBLE-CLICK' : button }
    }
    case 'scroll':
      return x === null || y === null
        ? { kind: 'none' }
        : {
            kind: 'scroll',
            x,
            y,
            label:
              `SCROLL ${(typeof args.direction === 'string' ? args.direction : '').toUpperCase()}`.trim(),
          }
    case 'type_text':
      return {
        kind: 'type',
        text: typeof args.text === 'string' ? (show ? args.text : hiddenText(args.text)) : '••••',
      }
    case 'press_keys':
      return Array.isArray(args.keys)
        ? { kind: 'keys', text: args.keys.map(String).join(' + ') }
        : { kind: 'none' }
    default:
      return { kind: 'none' }
  }
}

/** `STEP 3 / 6 · CLICK`. */
export function stepBadge(index: number, count: number, step: FlightStep | undefined): string {
  if (!step) return ''
  const tool = (step.tool ?? 'no action').replaceAll('_', ' ').toUpperCase()
  return `STEP ${index + 1} / ${count} · ${tool}`
}

/** The one sentence under the timeline: what the model said it was doing, and how it went. */
export function caption(step: FlightStep | undefined, show: boolean): string {
  if (!step) return ''
  const reason = guardText(step, step.reason, show).trim()
  const outcome =
    step.error && step.error.trim() !== ''
      ? `Failed: ${step.error.trim()}`
      : step.result === 'ok'
        ? ''
        : step.result
          ? `Result: ${step.result}`
          : ''
  return [reason, outcome].filter(Boolean).join(' ')
}

// --------------------------------------------------------------------------- the drawer

export type TabId =
  | 'step'
  | 'reasoning'
  | 'prompt'
  | 'safety'
  | 'api'
  | 'timing'
  | 'tools'
  | 'files'

export const TAB_LABEL: Record<TabId, string> = {
  step: 'Step',
  reasoning: 'Reasoning',
  prompt: 'Prompt',
  safety: 'Safety',
  api: 'API',
  timing: 'Timing',
  tools: 'Tools',
  files: 'Files',
}

export interface TabContext {
  step: FlightStep | undefined
  calls: FlightCall[]
  hasFiles: boolean
  /** Outside-agent `rune_*` calls recorded for the step. */
  toolEvents: number
}

/** The tabs that have something to show for the current step. A tab with no data is not listed. */
export function tabsFor({ step, calls, hasFiles, toolEvents }: TabContext): TabId[] {
  if (!step) return []
  const tabs: TabId[] = ['step']
  if (step.thinkingChars !== null || step.completionTokens !== null) tabs.push('reasoning')
  if (step.promptText) tabs.push('prompt')
  if (step.riskStated || step.riskFinal || step.gate) tabs.push('safety')
  if (calls.length > 0) tabs.push('api')
  if (phaseTimes(step.phases).some((item) => item.ms > 0)) tabs.push('timing')
  if (toolEvents > 0) tabs.push('tools')
  if (hasFiles) tabs.push('files')
  return tabs
}

/** The next tab in the list, wrapping round (the T key). */
export function nextTab(tabs: TabId[], current: TabId): TabId | null {
  if (tabs.length === 0) return null
  return tabs[(tabs.indexOf(current) + 1) % tabs.length] ?? null
}

// ------------------------------------------------------------------------ long lists

/** The rows to draw for a list of `count` fixed-height rows: the visible ones plus a little more. */
export function visibleRows(
  count: number,
  scrollTop: number,
  viewport: number,
  rowHeight: number,
  overscan = 4,
): { from: number; to: number } {
  if (count <= 0 || rowHeight <= 0) return { from: 0, to: 0 }
  const first = Math.floor(Math.max(0, scrollTop) / rowHeight)
  const last = Math.ceil((Math.max(0, scrollTop) + Math.max(0, viewport)) / rowHeight)
  return { from: Math.max(0, first - overscan), to: Math.min(count, last + overscan) }
}

/** The events that happened while a step was current (by time; events carry second-resolution stamps). */
export function eventsOfStep(
  events: FlightEvent[],
  runStartedUtc: string | null,
  steps: FlightStep[],
  index: number,
): FlightEvent[] {
  const origin = parseIso(runStartedUtc)
  const starts = absoluteStarts(runStartedUtc, steps)
  if (origin === null || index < 0 || index >= starts.length) return []
  const begin = starts[index] ?? 0
  const end = starts[index + 1] ?? Number.POSITIVE_INFINITY
  return events.filter((event) => {
    const at = parseIso(event.atUtc)
    return at !== null && at - origin >= begin && at - origin < end
  })
}

/** A short word and whether it is an error, for the markers drawn on the timeline. */
export function markerOf(event: FlightEvent): { word: string; error: boolean } | null {
  switch (event.kind) {
    case 'stuck_warning':
      return { word: 'STUCK', error: false }
    case 'steer':
      return { word: 'STEER', error: false }
    case 'ask_user':
      return { word: 'ASK', error: false }
    case 'invalid_answer':
      return { word: 'INVALID', error: true }
    case 'elevated_warning':
      return { word: 'CAUTION', error: false }
    default:
      return null
  }
}

/** The speeds the player offers. */
export const SPEEDS = [0.5, 1, 2, 4] as const

export function slower(speed: number): number {
  const index = SPEEDS.findIndex((value) => value >= speed)
  return SPEEDS[Math.max(0, (index === -1 ? SPEEDS.length : index) - 1)] ?? 1
}

export function faster(speed: number): number {
  const index = SPEEDS.findIndex((value) => value > speed)
  return (index === -1 ? SPEEDS[SPEEDS.length - 1] : SPEEDS[index]) ?? 4
}

// ------------------------------------------------------------------ the picture and the track

/** The largest box of a given shape (width / height) that fits inside a space. */
export function fitBox(
  width: number,
  height: number,
  aspect: number,
): { width: number; height: number } {
  if (!(width > 0) || !(height > 0) || !(aspect > 0)) return { width: 0, height: 0 }
  return width / height > aspect
    ? { width: Math.floor(height * aspect), height: Math.floor(height) }
    : { width: Math.floor(width), height: Math.floor(width / aspect) }
}

/** The segments that touch the visible part of the track (plus a margin), for a given zoom. */
export function visibleSegments(
  list: Segment[],
  pxPerMs: number,
  scrollLeft: number,
  viewport: number,
  margin = 120,
): Segment[] {
  if (!(pxPerMs > 0)) return []
  const from = scrollLeft - margin
  const to = scrollLeft + viewport + margin
  return list.filter((segment) => {
    const left = segment.start * pxPerMs
    const right = left + Math.max(segment.length * pxPerMs, 6)
    return right >= from && left <= to
  })
}

/** The time under a pointer on the track, given its scroll and zoom. */
export function timeAtPixel(x: number, scrollLeft: number, pxPerMs: number, total: number): number {
  if (!(pxPerMs > 0)) return 0
  return Math.min(total, Math.max(0, (x + scrollLeft) / pxPerMs))
}

export const ZOOM_MIN = 1
export const ZOOM_MAX = 40

/** One wheel notch zooms by a quarter step; the result stays inside the limits. */
export function zoomed(zoom: number, wheelDelta: number): number {
  const next = zoom * (wheelDelta < 0 ? 1.25 : 0.8)
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, next))
}

/** The icon (by meaning) drawn above a step on the track. */
export function toolIcon(
  tool: string | null,
): 'mouse' | 'keyboard' | 'shortcut' | 'wait' | 'scan' | 'eye' {
  switch (tool) {
    case 'click':
    case 'click_element':
    case 'scroll':
    case 'drag':
      return 'mouse'
    case 'type_text':
      return 'keyboard'
    case 'press_keys':
      return 'shortcut'
    case 'wait':
      return 'wait'
    case 'observe':
      return 'eye'
    default:
      return 'scan'
  }
}

/** Whether a step went wrong (the only thing drawn in red). */
export function failedStep(step: FlightStep): boolean {
  return step.result === 'failed' || (step.error ?? '').trim() !== ''
}

/** A run from the built-in agent, as opposed to one an outside agent drove through the bridge. */
export function isOutsideAgent(source: string): boolean {
  const name = source.trim().toLowerCase()
  return name !== '' && name !== 'built-in' && name !== 'rune'
}

// ------------------------------------------------------------------- hidden text, bodies

/** Everything the run typed with `type_text`, longest first (so a longer text is masked whole). */
export function typedTexts(steps: FlightStep[]): string[] {
  const found = new Set<string>()
  for (const step of steps) {
    if (!isTyping(step)) continue
    const text = argsOf(step).text
    if (typeof text === 'string' && text.length > 0) found.add(text)
  }
  return [...found].sort((a, b) => b.length - a.length)
}

/** `text` with anything the run typed replaced by a mask, unless the viewer chose SHOW. */
export function maskTyped(text: string, typed: string[], show: boolean): string {
  if (show || text === '') return text
  let out = text
  for (const item of typed) out = out.split(item).join(hiddenText(item))
  // The same text inside a JSON string is escaped; mask that spelling too.
  for (const item of typed) {
    const escaped = JSON.stringify(item).slice(1, -1)
    if (escaped !== item) out = out.split(escaped).join(hiddenText(item))
  }
  return out
}

/** JSON text laid out for reading; anything that is not JSON is returned as it is. */
export function pretty(text: string | null | undefined): string {
  if (!text) return ''
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
}

/** The model's reasoning text from a stored reply, when the server returned it. */
export function reasoningOf(responseJson: string | null | undefined): string | null {
  if (!responseJson) return null
  try {
    const reply: unknown = JSON.parse(responseJson)
    const message = (reply as { choices?: { message?: Record<string, unknown> }[] })?.choices?.[0]
      ?.message
    const text = message?.reasoning_content ?? message?.reasoning
    return typeof text === 'string' && text.trim() !== '' ? text : null
  } catch {
    return null
  }
}
/** One part of the message sent to the model: a heading and its lines. */
export interface PromptSection {
  title: string
  body: string
  lines: number
}

/**
 * The message sent to the model, split into its parts ("Task", "Plan", "Actions so far", ...) so it can
 * be read as a few labelled blocks instead of one wall of text. A part is a blank-line separated block
 * whose first line looks like `Heading:`; anything else is kept as its own untitled block.
 */
export function promptSections(text: string | null | undefined): PromptSection[] {
  if (!text) return []
  return text
    .split(/\n{2,}/)
    .map((block) => block.trim())
    .filter((block) => block !== '')
    .map((block) => {
      const heading = /^([A-Z][^:\n]{0,70}):[ \t]*(.*)$/.exec(block.split('\n')[0] ?? '')
      const rest = block.split('\n').slice(1).join('\n')
      const body = heading ? [heading[2], rest].filter((part) => part !== '').join('\n') : block
      return {
        title: heading?.[1] ?? 'Note',
        body,
        lines: body.split('\n').length,
      }
    })
}

/** Where a risk word sits on the three-step meter: 0 low, 1 medium, 2 high, -1 unknown. */
export function riskLevel(risk: string | null | undefined): number {
  const index = ['low', 'medium', 'high'].indexOf((risk ?? '').toLowerCase())
  return index
}