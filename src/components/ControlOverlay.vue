<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, usePreferredReducedMotion, useRafFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'

import { KIND_ICON, TOOL_LABEL, kindOf, type ActionKind } from '@/config/actions'
import type {
  AgentState,
  ObserveStartedPayload,
  Settings,
  StepStartedPayload,
} from '@/types/protocol'

import AppIcon from './AppIcon.vue'

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

  for (const frameDot of dots) {
    if (frameDot.depth > reveal) continue
    const edge = 1 - frameDot.depth / DEPTH
    let level = 0.1 + 0.34 * edge * edge
    if (head >= 0) {
      const behind = (head - frameDot.along + perimeter) % perimeter
      if (behind < TAIL) {
        const glow = 1 - behind / TAIL
        level = Math.max(level, glow * glow * (0.35 + 0.65 * edge))
      }
    }
    // A dark ring keeps the white dots readable over light windows.
    ctx.beginPath()
    ctx.arc(frameDot.x, frameDot.y, DOT + 0.9 + level, 0, Math.PI * 2)
    ctx.fillStyle = `rgba(0, 0, 0, ${0.22 * level + 0.06})`
    ctx.fill()
    ctx.beginPath()
    ctx.arc(frameDot.x, frameDot.y, DOT + 0.7 * level, 0, Math.PI * 2)
    ctx.fillStyle = `rgba(255, 255, 255, ${Math.min(1, level + 0.1)})`
    ctx.fill()
  }
  drawFlashes(ctx, now, still)
  drawRipples(ctx, now, still)
  updateTyping(now, still)
}

function dot(ctx: CanvasRenderingContext2D, x: number, y: number, radius: number, alpha: number) {
  ctx.beginPath()
  ctx.arc(x, y, radius + 1.2, 0, Math.PI * 2)
  ctx.fillStyle = `rgba(0, 0, 0, ${0.3 * alpha})`
  ctx.fill()
  ctx.beginPath()
  ctx.arc(x, y, radius, 0, Math.PI * 2)
  ctx.fillStyle = `rgba(255, 255, 255, ${alpha})`
  ctx.fill()
}

/** A click marker: rings of dots spreading from the point where the agent clicked. */
interface Ripple {
  x: number
  y: number
  at: number
  /** 1 or 2 pulses (a double click pulses twice). */
  pulses: number
}
const RIPPLE_SECONDS = 0.9
const RIPPLE_REACH = 72
let ripples: Ripple[] = []

/** Rings of dots on an 8 px pitch spreading out from each click, and a dot that stays at the centre. */
function drawRipples(ctx: CanvasRenderingContext2D, now: number, still: boolean) {
  ripples = ripples.filter(
    (ripple) => (now - ripple.at) / 1000 < RIPPLE_SECONDS * ripple.pulses + 0.4,
  )
  for (const ripple of ripples) {
    const age = (now - ripple.at) / 1000
    if (age < 0) continue
    for (let pulse = 0; pulse < ripple.pulses; pulse++) {
      const t = still ? 0.5 : (age - pulse * 0.35) / RIPPLE_SECONDS
      if (t < 0 || t > 1) continue
      const radius = 8 + t * RIPPLE_REACH
      const count = Math.max(8, Math.round((Math.PI * 2 * radius) / STEP))
      for (let i = 0; i < count; i++) {
        const angle = (i / count) * Math.PI * 2
        dot(
          ctx,
          ripple.x + Math.cos(angle) * radius,
          ripple.y + Math.sin(angle) * radius,
          1.6,
          (1 - t) * 0.9,
        )
      }
    }
    const settle = Math.max(0, 1 - Math.max(0, age - 0.3) / (RIPPLE_SECONDS * ripple.pulses + 0.1))
    dot(ctx, ripple.x, ripple.y, 3, Math.min(1, 0.35 + settle))
  }
}

/** The area a screenshot covered: a viewfinder of dot corners, a scan line of dots and a brief wash. */
interface Flash {
  x: number
  y: number
  w: number
  h: number
  at: number
}
const FLASH_SECONDS = 0.8
/** How much of the flash the scan line takes, the rest is the frame fading out. */
const SWEEP_SHARE = 0.7
let flashes: Flash[] = []

function drawFlashes(ctx: CanvasRenderingContext2D, now: number, still: boolean) {
  flashes = flashes.filter((flash) => (now - flash.at) / 1000 < FLASH_SECONDS)
  for (const flash of flashes) {
    const t = still ? 0.3 : Math.max(0, (now - flash.at) / 1000 / FLASH_SECONDS)
    // Quick brightening, then a slow fade.
    const envelope = t < 0.12 ? t / 0.12 : 1 - (t - 0.12) / 0.88
    ctx.fillStyle = `rgba(255, 255, 255, ${0.12 * envelope})`
    ctx.fillRect(flash.x, flash.y, flash.w, flash.h)

    if (t < SWEEP_SHARE) {
      const sweep = flash.y + (t / SWEEP_SHARE) * flash.h
      for (let row = 0; row < 4; row++) {
        const y = sweep - row * STEP
        if (y < flash.y) break
        for (let x = flash.x; x <= flash.x + flash.w; x += STEP)
          dot(ctx, x, y, 1.6, 0.9 * (1 - row / 4))
      }
    }

    // Corner brackets: an L of three dots at each corner.
    const alpha = Math.min(1, (1 - t) * 1.6)
    for (const [cx, cy, dx, dy] of [
      [flash.x, flash.y, 1, 1],
      [flash.x + flash.w, flash.y, -1, 1],
      [flash.x, flash.y + flash.h, 1, -1],
      [flash.x + flash.w, flash.y + flash.h, -1, -1],
    ] as const) {
      for (let i = 0; i < 4; i++) {
        dot(ctx, cx + dx * i * STEP, cy, 2.2, alpha)
        if (i > 0) dot(ctx, cx, cy + dy * i * STEP, 2.2, alpha)
      }
    }
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

/** Name of the outside agent in control, or null when it is Rune Studio's own agent. */
const agentName = ref<string | null>(null)

/** Characters of typed text shown at once; longer text scrolls so the newest letters stay in view. */
const TYPED_TAIL = 24

interface Typing {
  text: string
  startedAt: number
  msPerChar: number
}

/** What the agent is doing right now. One at a time; the next replaces it. */
const action = ref<{
  key: number
  kind: ActionKind
  label: string
  detail: string
  typing: Typing | null
} | null>(null)

/** How many characters of the text being typed have appeared so far. */
const typedCount = ref(0)
const typedShown = computed(() => {
  const text = action.value?.typing?.text.slice(0, typedCount.value).replaceAll('\n', '↵') ?? ''
  return text.length > TYPED_TAIL ? `…${text.slice(-TYPED_TAIL)}` : text
})

/** The caption types out in step with the real typing, so the user sees the text arrive. */
function updateTyping(now: number, still: boolean) {
  const typing = action.value?.typing
  if (!typing) return
  const count = still
    ? typing.text.length
    : Math.min(typing.text.length, Math.floor((now - typing.startedAt) / typing.msPerChar))
  if (count !== typedCount.value) typedCount.value = count
}

function showStep(step: StepStartedPayload) {
  const prefix =
    step.tool === 'click' ? (step.double ? 'Double ' : step.button === 'right' ? 'Right ' : '') : ''
  const typing =
    step.tool === 'type_text' && step.detail
      ? {
          text: step.detail,
          startedAt: performance.now(),
          msPerChar: step.durationMs > 0 ? step.durationMs / [...step.detail].length : 15,
        }
      : null
  typedCount.value = 0
  action.value = {
    key: step.step,
    kind: kindOf(step.tool),
    typing,
    label: prefix ? `${prefix}click` : (TOOL_LABEL[step.tool] ?? step.tool),
    detail: typing ? '' : step.detail.length > 28 ? `${step.detail.slice(0, 27)}…` : step.detail,
  }
  if (step.point) {
    // Desktop pixels to this window: the window sits at its monitor's origin.
    const dpr = window.devicePixelRatio || 1
    ripples.push({
      x: step.point.x / dpr - window.screenX,
      y: step.point.y / dpr - window.screenY,
      // The ripple lands when the cursor arrives, not when the step starts.
      at: performance.now() + step.durationMs,
      pulses: step.double ? 2 : 1,
    })
  }
}

let observeCount = 0

/** A screenshot: the monitor icon, and a flash over the area that was captured. */
function showObserve({ region }: ObserveStartedPayload) {
  typedCount.value = 0
  action.value = {
    key: -++observeCount,
    kind: 'screen',
    label: region ? 'Zoom' : TOOL_LABEL.observe,
    detail: '',
    typing: null,
  }
  // The whole screen is flashed a little inside the edge frame, so the two do not overlap.
  const inset = 3 * STEP
  const [x0, y0, x1, y1] = region ?? [0, 0, 1000, 1000]
  const box = region
    ? {
        x: (Math.min(x0, x1) / 1000) * width,
        y: (Math.min(y0, y1) / 1000) * height,
        w: (Math.abs(x1 - x0) / 1000) * width,
        h: (Math.abs(y1 - y0) / 1000) * height,
      }
    : { x: inset, y: inset, w: width - 2 * inset, h: height - 2 * inset }
  flashes.push({ ...box, at: performance.now() })
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
    await listen<{ state: AgentState }>('state_changed', (event) => {
      const next = event.payload.state
      setActive(next === 'acting' || next === 'observing')
      if (next !== 'acting' && next !== 'observing' && next !== 'planning') action.value = null
    }),
    await listen<StepStartedPayload>('step_started', (event) => showStep(event.payload)),
    await listen<ObserveStartedPayload>('observe_started', (event) => showObserve(event.payload)),
    await listen<{ agent: string | null }>(
      'external_agent',
      (event) => (agentName.value = event.payload.agent),
    ),
  )
  // The window is created at the moment control starts, so it may have missed the events.
  setActive(true)
  try {
    agentName.value = await invoke<string | null>('get_control_label')
  } catch {
    // The label falls back to Rune.
  }
  // Dev only: lets a plain browser preview play steps without the Rust side.
  if (import.meta.env.DEV) {
    Object.assign(window, { __demoStep: showStep, __demoObserve: showObserve })
  }
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

    <!-- One pill: who is in control, what they are doing right now, and how to stop them. -->
    <div
      class="absolute top-6 left-1/2 flex h-10 -translate-x-1/2 items-center gap-3 rounded-full bg-black/85 pr-1 pl-4 text-white"
    >
      <span class="size-2 shrink-0 animate-pulse rounded-full bg-[#d71921]" />
      <span class="text-caption whitespace-nowrap uppercase"
        >{{ agentName ?? 'Rune' }} is in control</span
      >

      <Transition name="action" mode="out-in">
        <div
          v-if="action"
          :key="action.key"
          class="flex items-center gap-2 border-l border-white/20 pl-3"
        >
          <AppIcon :name="KIND_ICON[action.kind]" />
          <span class="text-caption whitespace-nowrap uppercase">{{ action.label }}</span>
          <span v-if="action.detail" class="max-w-64 truncate text-label text-white/70">{{
            action.detail
          }}</span>
          <span
            v-if="action.typing"
            class="flex items-center text-label whitespace-pre text-white/70"
          >
            {{ typedShown }}<span class="caret ml-px h-4 w-[2px] bg-white" />
          </span>
        </div>
      </Transition>

      <div class="flex items-center gap-2 border-l border-white/20 pl-3">
        <span v-if="hotkeyLabel" class="text-caption whitespace-nowrap text-white/50 uppercase">{{
          hotkeyLabel
        }}</span>
        <span class="grid size-8 shrink-0 place-items-center rounded-full bg-[#d71921]">
          <AppIcon name="stop" />
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.caret {
  animation: caret-blink 0.9s steps(1) infinite;
}

@keyframes caret-blink {
  50% {
    opacity: 0;
  }
}

.action-enter-active,
.action-leave-active {
  transition:
    opacity 160ms ease,
    transform 160ms ease;
}

.action-enter-from,
.action-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

@media (prefers-reduced-motion: reduce) {
  .action-enter-active,
  .action-leave-active {
    transition: none;
  }

  .caret {
    animation: none;
  }
}
</style>