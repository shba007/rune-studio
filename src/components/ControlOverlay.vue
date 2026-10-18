<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, usePreferredReducedMotion, useRafFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'

import type { AgentState, Settings } from '@/types/protocol'

/** Full-screen "the agent is driving" effect: a dot-matrix frame with a pulse circling it. */

const STEP = 8
const DEPTH = 4
const DOT = 1.15
const TRAVEL = 520
const TAIL = 380

const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
const active = ref(false)
const stopHotkey = ref('')
const reducedMotion = usePreferredReducedMotion()
let width = 0
let height = 0
let perimeter = 1
let startedAt = 0
let dots: { x: number; y: number; depth: number; along: number }[] = []

const hotkeyLabel = computed(() => stopHotkey.value.replace(/\+/g, ' + '))

/** Lays the dots out once per size: a band of rows and columns along every edge. */
function layout() {
  const el = canvas.value
  if (!el) return
  width = window.innerWidth
  height = window.innerHeight
  perimeter = 2 * (width + height)
  const dpr = window.devicePixelRatio || 1
  el.width = width * dpr
  el.height = height * dpr
  el.getContext('2d')?.setTransform(dpr, 0, 0, dpr, 0, 0)

  dots = []
  const cols = Math.floor(width / STEP)
  const rows = Math.floor(height / STEP)
  const x0 = (width - (cols - 1) * STEP) / 2
  const y0 = (height - (rows - 1) * STEP) / 2
  for (let c = 0; c < cols; c++) {
    for (let r = 0; r < rows; r++) {
      const depth = Math.min(c, r, cols - 1 - c, rows - 1 - r)
      if (depth >= DEPTH) continue
      const x = x0 + c * STEP
      const y = y0 + r * STEP
      // Distance travelled clockwise from the top-left corner, along the nearest edge.
      let along: number
      if (depth === r && r <= rows - 1 - r) along = x
      else if (depth === cols - 1 - c) along = width + y
      else if (depth === rows - 1 - r) along = width + height + (width - x)
      else along = 2 * width + height + (height - y)
      dots.push({ x, y, depth, along })
    }
  }
}

function draw(now: number) {
  const ctx = canvas.value?.getContext('2d')
  if (!ctx) return
  ctx.clearRect(0, 0, width, height)
  const still = reducedMotion.value === 'reduce'
  const seconds = (now - startedAt) / 1000
  // The frame draws itself inward from the very edge when control starts.
  const reveal = still ? DEPTH : Math.min(DEPTH, seconds * 14)
  const head = still ? -1 : (seconds * TRAVEL) % perimeter

  for (const dot of dots) {
    if (dot.depth > reveal) continue
    const edge = 1 - dot.depth / DEPTH
    let level = 0.1 + 0.34 * edge * edge
    if (head >= 0) {
      const behind = (head - dot.along + perimeter) % perimeter
      if (behind < TAIL) {
        const glow = 1 - behind / TAIL
        level = Math.max(level, glow * glow * (0.35 + 0.65 * edge))
      }
    }
    // A dark ring keeps the white dots readable over light windows.
    ctx.beginPath()
    ctx.arc(dot.x, dot.y, DOT + 0.9 + level, 0, Math.PI * 2)
    ctx.fillStyle = `rgba(0, 0, 0, ${0.22 * level + 0.06})`
    ctx.fill()
    ctx.beginPath()
    ctx.arc(dot.x, dot.y, DOT + 0.7 * level, 0, Math.PI * 2)
    ctx.fillStyle = `rgba(255, 255, 255, ${Math.min(1, level + 0.1)})`
    ctx.fill()
  }
}

const { pause, resume } = useRafFn(({ timestamp }) => draw(timestamp), {
  immediate: false,
  fpsLimit: 30,
})

function setActive(next: boolean) {
  if (next === active.value) return
  active.value = next
  if (next) {
    startedAt = performance.now()
    resume()
  } else {
    // Keep painting while the page fades out, then stop.
    window.setTimeout(() => {
      if (!active.value) pause()
    }, 600)
  }
}

const unlisten: UnlistenFn[] = []

onMounted(async () => {
  layout()
  draw(performance.now())
  try {
    stopHotkey.value = (await invoke<Settings>('get_settings')).stopHotkey
  } catch {
    // The label is optional.
  }
  unlisten.push(
    await listen<{ state: AgentState }>('state_changed', (event) =>
      setActive(event.payload.state === 'acting' || event.payload.state === 'observing'),
    ),
  )
  // The window is created at the moment control starts, so it may have missed the event.
  setActive(true)
})
useEventListener(window, 'resize', layout)
onBeforeUnmount(() => unlisten.forEach((off) => off()))
</script>

<template>
  <div
    class="fixed inset-0 transition-opacity duration-500 ease-out"
    :class="active ? 'opacity-100' : 'opacity-0'"
  >
    <canvas ref="canvas" class="absolute inset-0 size-full" aria-hidden="true" />
    <div
      class="absolute top-6 left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-full bg-black/85 py-2 pr-4 pl-4 text-white"
    >
      <span class="size-2 animate-pulse rounded-full bg-[#d71921]" />
      <span class="text-caption uppercase">Rune is in control</span>
      <span v-if="hotkeyLabel" class="text-caption text-white/50 uppercase"
        >· Stop {{ hotkeyLabel }}</span
      >
    </div>
  </div>
</template>