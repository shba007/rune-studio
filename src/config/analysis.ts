import type {
  AnalysisSnapshot,
  AnalysisStatus,
  Box,
  Finding,
  FindingKind,
  MonitorInfo,
} from '@/types/protocol'

const STATUSES: AnalysisStatus[] = ['idle', 'capturing', 'asking', 'done', 'error', 'cancelled']
const KINDS: FindingKind[] = ['text', 'control', 'image', 'region', 'issue', 'other']

export const EMPTY_SNAPSHOT: AnalysisSnapshot = {
  status: 'idle',
  id: '',
  question: '',
  answer: '',
  findings: [],
  model: '',
  latencyMs: 0,
  error: null,
  createdUtc: '',
  monitor: null,
  stale: false,
  boxesRejected: 0,
  selected: null,
}

const text = (value: unknown) => (typeof value === 'string' ? value : '')
const count = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) ? value : 0)

function readBox(value: unknown): Box | null {
  if (!Array.isArray(value) || value.length !== 4) return null
  if (!value.every((n) => typeof n === 'number' && Number.isFinite(n))) return null
  return value as Box
}

function readPoint(value: unknown): [number, number] | null {
  if (!Array.isArray(value) || value.length !== 2) return null
  if (!value.every((n) => typeof n === 'number' && Number.isFinite(n))) return null
  return value as [number, number]
}

function readFinding(value: unknown, index: number): Finding {
  const raw = (typeof value === 'object' && value !== null ? value : {}) as Record<string, unknown>
  const kind = KINDS.find((known) => known === raw.kind) ?? 'other'
  return {
    n: count(raw.n) || index + 1,
    label: text(raw.label),
    kind,
    box: readBox(raw.box),
    description: text(raw.description),
    evidence: text(raw.evidence),
    point: readPoint(raw.point),
  }
}

function readMonitor(value: unknown): MonitorInfo | null {
  if (typeof value !== 'object' || value === null) return null
  const raw = value as Record<string, unknown>
  const width = count(raw.width)
  const height = count(raw.height)
  if (width <= 0 || height <= 0) return null
  return {
    id: count(raw.id),
    x: count(raw.x),
    y: count(raw.y),
    width,
    height,
    scaleFactor: count(raw.scaleFactor) || 1,
    primary: raw.primary === true,
  }
}

/** Reads what the backend sent, whatever it is: unknown fields are ignored, missing ones get defaults. */
export function readSnapshot(raw: unknown): AnalysisSnapshot {
  if (typeof raw !== 'object' || raw === null) return { ...EMPTY_SNAPSHOT }
  const value = raw as Record<string, unknown>
  return {
    status: STATUSES.find((known) => known === value.status) ?? 'idle',
    id: text(value.id),
    question: text(value.question),
    answer: text(value.answer),
    findings: Array.isArray(value.findings) ? value.findings.map(readFinding) : [],
    model: text(value.model),
    latencyMs: count(value.latencyMs),
    error: typeof value.error === 'string' ? value.error : null,
    createdUtc: text(value.createdUtc),
    monitor: readMonitor(value.monitor),
    stale: value.stale === true,
    boxesRejected: count(value.boxesRejected),
    selected: typeof value.selected === 'number' ? value.selected : null,
  }
}

// ---------------------------------------------------------------- the screen markers

/** The overlay window as the page knows it: its size in physical pixels and the display scale. */
export interface OverlayView {
  width: number
  height: number
  scaleFactor: number
}

export interface CssRect {
  left: number
  top: number
  width: number
  height: number
}

/**
 * A finding's box (0..1000 across the whole captured screen) as CSS pixels inside the overlay window,
 * which covers its monitor. The window's CSS size is its physical size over the display scale, so the
 * result is right at 100, 125 and 150 percent; the monitor's origin on the desktop does not matter
 * (the window sits at it).
 */
export function boxToCss(box: Box, view: OverlayView): CssRect {
  const scale = view.scaleFactor > 0 ? view.scaleFactor : 1
  const cssWidth = view.width / scale
  const cssHeight = view.height / scale
  const [x0, y0, x1, y1] = box
  return {
    left: (x0 / 1000) * cssWidth,
    top: (y0 / 1000) * cssHeight,
    width: ((x1 - x0) / 1000) * cssWidth,
    height: ((y1 - y0) / 1000) * cssHeight,
  }
}

/** Where the numbered badge goes: centred on the box's top-left corner, kept inside the window. */
export function badgePosition(
  rect: CssRect,
  view: OverlayView,
  size = 20,
): { x: number; y: number } {
  const scale = view.scaleFactor > 0 ? view.scaleFactor : 1
  const clamp = (value: number, max: number) =>
    Math.min(Math.max(value, 0), Math.max(0, max - size))
  return {
    x: clamp(rect.left - size / 2, view.width / scale),
    y: clamp(rect.top - size / 2, view.height / scale),
  }
}

/** The findings that have a box to draw on the screen (a finding without one has no marker). */
export function drawable(snapshot: AnalysisSnapshot): Finding[] {
  if (snapshot.stale) return []
  return snapshot.findings.filter((finding) => finding.box !== null)
}

/** How strongly a marker is drawn: full for the picked one (or when nothing is picked), dimmed otherwise. */
export function markerOpacity(n: number, selected: number | null): number {
  return selected === null || selected === n ? 1 : 0.35
}

// ---------------------------------------------------------------- the status bar and the cards

export type StageWord = 'LOOKING' | 'READING' | 'DONE' | 'CANCELLED' | 'FAILED'

/** The stage chip of the status bar; null when there is nothing to say. */
export function stageWord(status: AnalysisStatus): StageWord | null {
  switch (status) {
    case 'capturing':
      return 'LOOKING'
    case 'asking':
      return 'READING'
    case 'done':
      return 'DONE'
    case 'cancelled':
      return 'CANCELLED'
    case 'error':
      return 'FAILED'
    default:
      return null
  }
}

/** Elapsed time as `mm:ss`, the way the bar shows it (`00:14`). */
export function formatElapsed(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  const minutes = Math.floor(seconds / 60)
  return `${String(minutes).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`
}

/** The accordion: one finding open at a time; opening the open one closes it. */
export function toggleOpen(open: number | null, clicked: number): number | null {
  return open === clicked ? null : clicked
}

export interface CardsState {
  /** The finding row that is expanded. */
  open: number | null
  /** The finding whose marker is picked. */
  selected: number | null
  /** The cards are showing. */
  shown: boolean
}

/** What Escape does: collapse the expanded finding, then clear the selection, then close the cards. */
export function escape(state: CardsState): CardsState {
  if (state.open !== null) return { ...state, open: null }
  if (state.selected !== null) return { ...state, selected: null }
  return { ...state, shown: false }
}