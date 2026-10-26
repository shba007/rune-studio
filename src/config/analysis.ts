import type {
  AnalysisGroup,
  AnalysisSnapshot,
  AnalysisStatus,
  Box,
  Finding,
  FindingKind,
  MonitorInfo,
  Pose,
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
    pose: raw.group === 'structure' ? readPose(raw.pose) : null,
    n: count(raw.n) || index + 1,
    label: text(raw.label),
    kind,
    group: ['entities', 'regions', 'structure', 'depth'].includes(String(raw.group))
      ? (raw.group as AnalysisGroup)
      : kind === 'region'
        ? 'regions'
        : 'entities',
    box: readBox(raw.box),
    description: text(raw.description),
    evidence: text(raw.evidence),
    point: readPoint(raw.point),
  }
}

export function readPose(value: unknown): Pose | null {
  if (!value || typeof value !== 'object') return null
  const raw = value as Record<string, unknown>
  if (
    !Array.isArray(raw.keypoints) ||
    !raw.keypoints.length ||
    raw.keypoints.length > 32 ||
    !Array.isArray(raw.connections) ||
    raw.connections.length > 64
  )
    return null
  if (
    !raw.keypoints.every((p: unknown) => {
      if (!p || typeof p !== 'object') return false
      const point = p as Record<string, unknown>
      return (
        typeof point.label === 'string' &&
        point.label.trim().length > 0 &&
        Array.from(point.label).length <= 24 &&
        [point.x, point.y].every(
          (n) => typeof n === 'number' && Number.isFinite(n) && n >= 0 && n <= 1000,
        )
      )
    })
  )
    return null
  const length = raw.keypoints.length
  if (
    !raw.connections.every(
      (edge: unknown) =>
        Array.isArray(edge) &&
        edge.length === 2 &&
        edge.every((n) => Number.isInteger(n) && n >= 0 && n < length) &&
        edge[0] !== edge[1],
    )
  )
    return null
  return raw as unknown as Pose
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

export function readSnapshot(raw: unknown): AnalysisSnapshot {
  if (typeof raw !== 'object' || raw === null) return { ...EMPTY_SNAPSHOT }
  const value = raw as Record<string, unknown>
  return {
    screenshot:
      typeof value.screenshot === 'string' && value.screenshot.startsWith('data:image/jpeg;base64,')
        ? value.screenshot
        : null,
    visibleGroups: readVisibility(value.visibleGroups),
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

export interface OverlayView {
  width: number
  height: number
  scaleFactor: number
  originX?: number
  originY?: number
}

export interface CssRect {
  left: number
  top: number
  width: number
  height: number
}

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

export function drawable(snapshot: AnalysisSnapshot): Finding[] {
  if (snapshot.stale) return []
  return snapshot.findings.filter(
    (finding) =>
      (finding.box !== null || readPose(finding.pose) !== null) &&
      snapshot.visibleGroups?.[
        finding.group ?? (finding.kind === 'region' ? 'regions' : 'entities')
      ] !== false,
  )
}

export function markerOpacity(n: number, selected: number | null): number {
  return selected === null || selected === n ? 1 : 0.35
}

export type StageWord = 'LOOKING' | 'READING' | 'DONE' | 'CANCELLED' | 'FAILED'

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

export function formatElapsed(ms: number): string {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  const minutes = Math.floor(seconds / 60)
  return `${String(minutes).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`
}

export function toggleOpen(open: number | null, clicked: number): number | null {
  return open === clicked ? null : clicked
}

export interface CardsState {
  open: number | null
  selected: number | null
  shown: boolean
}

export function escape(state: CardsState): CardsState {
  if (state.open !== null) return { ...state, open: null }
  if (state.selected !== null) return { ...state, selected: null }
  return { ...state, shown: false }
}
function readVisibility(value: unknown) {
  const raw = (typeof value === 'object' && value !== null ? value : {}) as Record<string, unknown>
  return {
    entities: raw.entities !== false,
    regions: raw.regions !== false,
    structure: raw.structure !== false,
    depth: raw.depth !== false,
  }
}