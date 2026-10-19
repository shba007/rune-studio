import type { AnalysisSnapshot, AnalysisStatus, Box, Finding, FindingKind } from '@/types/protocol'

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
  image: null,
  width: 0,
  height: 0,
}

const text = (value: unknown) => (typeof value === 'string' ? value : '')
const count = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) ? value : 0)

function readBox(value: unknown): Box | null {
  if (!Array.isArray(value) || value.length !== 4) return null
  if (!value.every((n) => typeof n === 'number' && Number.isFinite(n))) return null
  return value as Box
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
    image: typeof value.image === 'string' ? value.image : null,
    width: count(value.width),
    height: count(value.height),
  }
}

// ---------------------------------------------------------------- label chips

/** What the chips are laid out for: the pixel size of the picture on screen. */
export interface Stage {
  width: number
  height: number
}

export interface ChipInput {
  n: number
  label: string
  box: Box
}

/** A chip and its leader line, in 0..1000 units like the boxes. `side` is where the chip sits. */
export interface ChipLayout {
  n: number
  x: number
  y: number
  w: number
  h: number
  side: 'right' | 'left' | 'inside'
  /** From the chip's edge to the box's edge; null for a chip drawn inside its box. */
  leader: { x1: number; y1: number; x2: number; y2: number } | null
}

/** Chip height in screen pixels, and the pixels one letter and the padding take. */
const CHIP_HEIGHT_PX = 22
const CHIP_LETTER_PX = 7.5
const CHIP_PADDING_PX = 32
const CHIP_MAX_PX = 220
/** Space between a box and its chip, and between two stacked chips, in units. */
const GAP = 14
const STACK_GAP = 6

const clamp = (value: number, low: number, high: number) => Math.min(Math.max(value, low), high)
const overlapsAcross = (a: ChipLayout, b: ChipLayout) => a.x < b.x + b.w && b.x < a.x + a.w

/**
 * Puts a label chip beside each box, on the right when it fits and else on the left (else inside
 * the box), and stacks chips that would sit on top of each other. Pure and deterministic: the
 * same boxes always give the same layout, chips stay inside the picture, and chips that share
 * horizontal space never overlap (up to what fits: about forty rows of 22 px on a 1000 px picture).
 */
export function layoutChips(items: ChipInput[], stage: Stage): ChipLayout[] {
  if (stage.width <= 0 || stage.height <= 0) return []
  const h = (CHIP_HEIGHT_PX / stage.height) * 1000
  const placed: ChipLayout[] = items.map((item) => {
    const [x0, y0, x1] = item.box
    const wPx = Math.min(CHIP_MAX_PX, item.label.length * CHIP_LETTER_PX + CHIP_PADDING_PX)
    const w = Math.min(1000, (wPx / stage.width) * 1000)
    let side: ChipLayout['side'] = 'right'
    let x = x1 + GAP
    if (x + w > 1000) {
      side = 'left'
      x = x0 - GAP - w
      if (x < 0) {
        side = 'inside'
        x = clamp(x0, 0, 1000 - w)
      }
    }
    const y = clamp(y0, 0, 1000 - h)
    return { n: item.n, x, y, w, h, side, leader: null }
  })

  // Top to bottom, each chip pushed below the ones above it that share its columns.
  const order = [...placed].sort((a, b) => a.y - b.y || a.n - b.n)
  order.forEach((chip, at) => {
    for (const above of order.slice(0, at)) {
      if (overlapsAcross(chip, above)) chip.y = Math.max(chip.y, above.y + above.h + STACK_GAP)
    }
  })
  // What ran off the bottom is pulled back up, bottom to top.
  for (let at = order.length - 1; at >= 0; at--) {
    const chip = order[at]!
    let limit = 1000 - chip.h
    for (const below of order.slice(at + 1)) {
      if (overlapsAcross(chip, below)) limit = Math.min(limit, below.y - chip.h - STACK_GAP)
    }
    chip.y = Math.max(0, Math.min(chip.y, limit))
  }

  for (const chip of placed) {
    const box = items.find((item) => item.n === chip.n)!.box
    if (chip.side === 'inside') continue
    const middle = chip.y + chip.h / 2
    const toY = clamp(middle, box[1], box[3])
    chip.leader =
      chip.side === 'right'
        ? { x1: chip.x, y1: middle, x2: box[2], y2: toY }
        : { x1: chip.x + chip.w, y1: middle, x2: box[0], y2: toY }
  }
  return placed
}