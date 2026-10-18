<script setup lang="ts">
import { onBeforeUnmount, onMounted, useTemplateRef, watch } from 'vue'

import type { AgentState } from '@/types/protocol'

const props = defineProps<{ state: AgentState; level?: number }>()

type Kind = 'idle' | 'listening' | 'thinking' | 'acting' | 'attention' | 'done' | 'failed'

const KIND: Record<AgentState, Kind> = {
  idle: 'idle',
  stopped: 'idle',
  listening: 'listening',
  planning: 'thinking',
  observing: 'thinking',
  acting: 'acting',
  awaiting_confirm: 'attention',
  done: 'done',
  failed: 'failed',
}

const SIZE = 64
const GRID = 11
const CENTER = 5
const STEP = 5.4
const OFFSET = (SIZE - (GRID - 1) * STEP) / 2
const CHECK = [
  [2, 5],
  [3, 6],
  [4, 7],
  [5, 6],
  [6, 5],
  [7, 4],
  [8, 3],
] as const

const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
let frameId = 0
let smoothedLevel = 0
let kindStartedAt = performance.now()
const origin = performance.now()
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)')

const wave = (x: number) => Math.sin(x) * 0.5 + 0.5

type Light = (x: number, y: number, brightness: number) => void

/** Brightness of every dot for one frame. `since` is seconds since the current state began. */
function paint(kind: Kind, t: number, since: number, light: Light) {
  for (let x = 0; x < GRID; x++) {
    for (let y = 0; y < GRID; y++) {
      const dx = x - CENTER
      const dy = y - CENTER
      const r = Math.hypot(dx, dy)
      const angle = Math.atan2(dy, dx)

      switch (kind) {
        case 'idle':
          if (Math.abs(r - 4) < 0.7) light(x, y, 0.25 + 0.25 * wave(t * 1.4))
          if (r < 0.5) light(x, y, 0.7)
          break
        case 'listening': {
          // Follows the real microphone level. At silence only the centre breathes, so a dead
          // mic looks different from a live one.
          const reach = 0.9 + smoothedLevel * 4.3
          if (r <= reach) light(x, y, 0.4 + 0.6 * (1 - r / reach))
          else if (Math.abs(r - 4.6) < 0.5) light(x, y, 0.08 + 0.25 * smoothedLevel)
          if (r < 0.5) light(x, y, 0.5 + 0.4 * wave(t * 2.5))
          break
        }
        case 'thinking':
          if (r > 3.2 && r < 4.8) {
            const head = (t * 2.4) % (Math.PI * 2)
            const behind = (angle - head + Math.PI * 4) % (Math.PI * 2)
            light(x, y, Math.max(0.06, 1 - behind / (Math.PI * 1.2)))
          }
          if (r < 0.6) light(x, y, 0.3 + 0.5 * wave(t * 4))
          break
        case 'acting': {
          if (Math.abs(r - 4.4) < 0.6) light(x, y, 0.14)
          const cursorX = 3 * Math.sin(t * 1.3)
          const cursorY = 2.5 * Math.sin(t * 2.1 + 1)
          const d = Math.hypot(dx - cursorX, dy - cursorY)
          light(x, y, Math.max(0, 1 - d / 1.1))
          const ripple = ((t % 2.4) / 2.4) * 5
          if (Math.abs(d - ripple) < 0.6) light(x, y, 0.4 * (1 - ripple / 5))
          break
        }
        case 'attention': {
          const pulse = Math.sin(t * 5) > 0 ? 1 : 0.3
          if (x === CENTER && ((y >= 2 && y <= 6) || y === 8)) light(x, y, 0.55 + 0.45 * pulse)
          else if (Math.abs(r - 4.7) < 0.5) light(x, y, 0.12 + 0.2 * pulse)
          break
        }
        case 'done':
          CHECK.forEach(([cx, cy], n) => {
            if (x === cx && y === cy && since > n * 0.07) light(x, y, 1)
          })
          if (Math.abs(r - 4.8) < 0.5) light(x, y, 0.1)
          break
        case 'failed':
          if (Math.abs(dx) === Math.abs(dy) && Math.abs(dx) <= 3) light(x, y, 1)
          break
      }
    }
  }
}

function draw(now: number) {
  const ctx = canvas.value?.getContext('2d')
  if (!ctx) return

  const kind = KIND[props.state]
  // Fast attack, slow decay: syllables register but the glyph does not flicker.
  smoothedLevel = Math.max(props.level ?? 0, smoothedLevel * 0.86)
  const t = reducedMotion.matches ? 0 : (now - origin) / 1000
  const since = reducedMotion.matches ? 10 : (now - kindStartedAt) / 1000
  const lit = new Map<number, number>()
  paint(kind, t, since, (x, y, b) => {
    if (x < 0 || y < 0 || x >= GRID || y >= GRID || b <= 0.02) return
    const key = x * GRID + y
    lit.set(key, Math.max(lit.get(key) ?? 0, b))
  })

  ctx.clearRect(0, 0, SIZE, SIZE)
  const red = kind === 'attention' || kind === 'failed'
  for (let x = 0; x < GRID; x++) {
    for (let y = 0; y < GRID; y++) {
      if (Math.hypot(x - CENTER, y - CENTER) > 5.2) continue
      const px = OFFSET + x * STEP
      const py = OFFSET + y * STEP
      ctx.beginPath()
      ctx.arc(px, py, 1.25, 0, Math.PI * 2)
      ctx.fillStyle = 'rgba(0, 0, 0, 0.13)'
      ctx.fill()

      const b = Math.min(1, lit.get(x * GRID + y) ?? 0)
      if (b > 0) {
        ctx.beginPath()
        ctx.arc(px, py, 1.25 + 0.95 * b, 0, Math.PI * 2)
        ctx.fillStyle = red ? `rgba(215, 25, 33, ${b})` : `rgba(17, 17, 17, ${b})`
        ctx.fill()
      }
    }
  }
}

function loop(now: number) {
  if (!document.hidden && !reducedMotion.matches) draw(now)
  frameId = requestAnimationFrame(loop)
}

watch(
  () => props.state,
  () => {
    kindStartedAt = performance.now()
    draw(kindStartedAt)
  },
)

onMounted(() => {
  const el = canvas.value
  if (!el) return
  const dpr = window.devicePixelRatio || 1
  el.width = SIZE * dpr
  el.height = SIZE * dpr
  el.getContext('2d')?.scale(dpr, dpr)
  draw(performance.now())
  frameId = requestAnimationFrame(loop)
})

onBeforeUnmount(() => cancelAnimationFrame(frameId))
</script>

<template>
  <canvas ref="canvas" :width="SIZE" :height="SIZE" class="block size-16" aria-hidden="true" />
</template>