export interface FlightRun {
  id: string
  task: string
  source: string
  outcome: string | null
  startedUtc: string | null
  endedUtc: string | null
  steps: number
  verdict: string | null
  thumb: string | null
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

export function parseIso(text: string | null | undefined): number | null {
  if (!text) return null
  const value = Date.parse(text)
  return Number.isFinite(value) ? value : null
}

export function stepStarts(runStartedUtc: string | null, steps: FlightStep[]): number[] {
  const absolute = absoluteStarts(runStartedUtc, steps)
  const shift = absolute[0] ?? 0
  return absolute.map((start) => start - shift)
}

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

export function timeOfStep(starts: number[], index: number): number {
  if (starts.length === 0) return 0
  return starts[Math.min(starts.length - 1, Math.max(0, index))] ?? 0
}

export function totalMs(starts: number[], header: Pick<FlightHeader, 'durationSecs'>): number {
  const last = starts.at(-1) ?? 0
  return Math.max((header.durationSecs ?? 0) * 1000, last + 1000)
}

export interface Segment {
  index: number
  start: number
  length: number
}

export function segments(starts: number[], total: number): Segment[] {
  return starts.map((start, index) => ({
    index,
    start,
    length: Math.max(0, (starts[index + 1] ?? total) - start),
  }))
}

export const PHASES = ['observe', 'decide', 'gate', 'act', 'settle'] as const
export type Phase = (typeof PHASES)[number]

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

export function clock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const seconds = String(total % 60).padStart(2, '0')
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, '0')}:${seconds}`
    : `${minutes}:${seconds}`
}

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

export const TYPING_TOOLS = ['type_text'] as const

export function isTyping(step: Pick<FlightStep, 'tool'>): boolean {
  return TYPING_TOOLS.some((name) => name === step.tool)
}

export function hiddenText(text: string): string {
  const count = Array.from(text).length
  return `•••• (${count} ${count === 1 ? 'character' : 'characters'})`
}

export function argsText(step: FlightStep, show: boolean): string {
  if (!step.argsJson) return ''
  if (!isTyping(step) || show) return step.argsJson
  const args = argsOf(step)
  if (typeof args.text === 'string') return JSON.stringify({ ...args, text: hiddenText(args.text) })
  return JSON.stringify({ ...args, text: '••••' })
}

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

export function stepBadge(index: number, count: number, step: FlightStep | undefined): string {
  if (!step) return ''
  const tool = (step.tool ?? 'no action').replaceAll('_', ' ').toUpperCase()
  return `STEP ${index + 1} / ${count} · ${tool}`
}

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
  toolEvents: number
}

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

export function nextTab(tabs: TabId[], current: TabId): TabId | null {
  if (tabs.length === 0) return null
  return tabs[(tabs.indexOf(current) + 1) % tabs.length] ?? null
}

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

export const SPEEDS = [0.5, 1, 2, 4] as const

export function slower(speed: number): number {
  const index = SPEEDS.findIndex((value) => value >= speed)
  return SPEEDS[Math.max(0, (index === -1 ? SPEEDS.length : index) - 1)] ?? 1
}

export function faster(speed: number): number {
  const index = SPEEDS.findIndex((value) => value > speed)
  return (index === -1 ? SPEEDS[SPEEDS.length - 1] : SPEEDS[index]) ?? 4
}

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

export function timeAtPixel(x: number, scrollLeft: number, pxPerMs: number, total: number): number {
  if (!(pxPerMs > 0)) return 0
  return Math.min(total, Math.max(0, (x + scrollLeft) / pxPerMs))
}

export const ZOOM_MIN = 1
export const ZOOM_MAX = 40

export function zoomed(zoom: number, wheelDelta: number): number {
  const next = zoom * (wheelDelta < 0 ? 1.25 : 0.8)
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, next))
}

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

export function failedStep(step: FlightStep): boolean {
  return step.result === 'failed' || (step.error ?? '').trim() !== ''
}

export function isOutsideAgent(source: string): boolean {
  const name = source.trim().toLowerCase()
  return name !== '' && name !== 'built-in' && name !== 'rune'
}

export function typedTexts(steps: FlightStep[]): string[] {
  const found = new Set<string>()
  for (const step of steps) {
    if (!isTyping(step)) continue
    const text = argsOf(step).text
    if (typeof text === 'string' && text.length > 0) found.add(text)
  }
  return [...found].sort((a, b) => b.length - a.length)
}

export function maskTyped(text: string, typed: string[], show: boolean): string {
  if (show || text === '') return text
  let out = text
  for (const item of typed) out = out.split(item).join(hiddenText(item))
  for (const item of typed) {
    const escaped = JSON.stringify(item).slice(1, -1)
    if (escaped !== item) out = out.split(escaped).join(hiddenText(item))
  }
  return out
}

export function pretty(text: string | null | undefined): string {
  if (!text) return ''
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
}

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
export interface PromptSection {
  title: string
  body: string
  lines: number
}

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

export function riskLevel(risk: string | null | undefined): number {
  const index = ['low', 'medium', 'high'].indexOf((risk ?? '').toLowerCase())
  return index
}