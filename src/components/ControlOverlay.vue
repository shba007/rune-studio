<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, usePreferredReducedMotion, useRafFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'

import { KIND_ICON, TOOL_LABEL, kindOf, type ActionKind } from '@/config/actions'
import type {
  AgentState,
  ObserveStartedPayload,
  PillRect,
  Settings,
  StepStartedPayload,
  ThinkingPayload,
  TypedPayload,
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
  /** Typed text rather than a click: a smaller ring around a caret mark. */
  typing?: boolean
}
const RIPPLE_SECONDS = 0.9
const RIPPLE_REACH = 72
const TYPING_REACH = 44
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
      const radius = 8 + t * (ripple.typing ? TYPING_REACH : RIPPLE_REACH)
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
    const hold = Math.min(1, 0.35 + settle)
    if (ripple.typing) {
      // A text caret: a short vertical bar of dots where the text went.
      for (const offset of [-1, 0, 1]) dot(ctx, ripple.x, ripple.y + offset * STEP, 2.2, hold)
    } else {
      dot(ctx, ripple.x, ripple.y, 3, hold)
    }
  }
}

/**
 * The area a screenshot covered: two runners of dots start at the top-left corner and trace the
 * outline of the captured area, meet at the opposite corner, hold a moment and fade. Calm: no
 * scanning and no wash over the screen, only a dotted outline that says "this is what I saw".
 */
interface Flash {
  x: number
  y: number
  w: number
  h: number
  at: number
}
const FLASH_SECONDS = 0.9
/** The outline is fully traced after this share of the flash. */
const TRACE_SHARE = 0.4
/** The outline starts to fade after this share. */
const HOLD_SHARE = 0.55
let flashes: Flash[] = []

/** A point `d` px clockwise from the top-left corner along a rectangle's outline. */
function onOutline(flash: Flash, d: number): [number, number] {
  const { x, y, w, h } = flash
  if (d <= w) return [x + d, y]
  if (d <= w + h) return [x + w, y + (d - w)]
  if (d <= 2 * w + h) return [x + w - (d - w - h), y + h]
  return [x, y + h - (d - 2 * w - h)]
}

function drawFlashes(ctx: CanvasRenderingContext2D, now: number, still: boolean) {
  flashes = flashes.filter((flash) => (now - flash.at) / 1000 < FLASH_SECONDS)
  for (const flash of flashes) {
    const t = still ? HOLD_SHARE : Math.max(0, (now - flash.at) / 1000 / FLASH_SECONDS)
    const reach = Math.min(1, t / TRACE_SHARE)
    const fade = t < HOLD_SHARE ? 1 : Math.max(0, 1 - (t - HOLD_SHARE) / (1 - HOLD_SHARE))
    const outline = 2 * (flash.w + flash.h)
    const count = Math.max(8, Math.round(outline / STEP))
    for (let i = 0; i < count; i++) {
      const d = (i / count) * outline
      // Each runner has travelled `reach` of the way to the far corner.
      if (Math.min(d, outline - d) > (reach * outline) / 2) continue
      const [px, py] = onOutline(flash, d)
      dot(ctx, px, py, 2, fade * 0.95)
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

/** Text was typed: mark the spot the way a click is marked, with a caret. */
function showTyped({ point }: TypedPayload) {
  if (!point) return
  const dpr = window.devicePixelRatio || 1
  ripples.push({
    x: point.x / dpr - window.screenX,
    y: point.y / dpr - window.screenY,
    at: performance.now(),
    pulses: 1,
    typing: true,
  })
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

/**
 * The speech bubble at the orb's head: what the agent is thinking (when Show thinking is on) and
 * what it just said. Compact here; the full text is in the pill's task panel when it is expanded.
 */
const BUBBLE_CHARS = 120
const bubbleThinking = ref('')
const bubbleSays = ref('')
const pill = ref<PillRect | null>(null)

async function refreshPill() {
  try {
    pill.value = await invoke<PillRect | null>('pill_rect')
  } catch {
    pill.value = null
  }
}

/** Thinking shows its newest words; a response shows its start. */
const bubble = computed(() => {
  // Expanded: the task panel shows all of it, so the bubble steps aside.
  if (!pill.value || pill.value.width > 100) return null
  if (bubbleThinking.value) {
    const text = bubbleThinking.value.replace(/\s+/g, ' ').trim()
    const shown = text.length > BUBBLE_CHARS ? `…${text.slice(-BUBBLE_CHARS)}` : text
    return { label: 'Thinking', live: true, text: shown }
  }
  if (bubbleSays.value) {
    const text = bubbleSays.value.replace(/\s+/g, ' ').trim()
    const shown = text.length > BUBBLE_CHARS ? `${text.slice(0, BUBBLE_CHARS)}…` : text
    return { label: 'Says', live: false, text: shown }
  }
  return null
})

/** Where the bubble sits: right edges aligned with the orb, just above its top. */
const bubbleStyle = computed(() => {
  const rect = pill.value
  if (!rect) return {}
  const dpr = window.devicePixelRatio || 1
  const right = (rect.x + rect.width) / dpr - window.screenX
  const top = rect.y / dpr - window.screenY
  return {
    right: `${Math.max(8, window.innerWidth - right)}px`,
    bottom: `${Math.max(8, window.innerHeight - top + 12)}px`,
  }
})

function onThinking({ text, done }: ThinkingPayload) {
  bubbleThinking.value = done ? '' : text
  void refreshPill()
}

function onSays(text: string) {
  bubbleThinking.value = ''
  bubbleSays.value = text
  void refreshPill()
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
      setActive(next === 'acting' || next === 'observing' || next === 'planning')
      if (next !== 'acting' && next !== 'observing' && next !== 'planning') action.value = null
      if (next === 'planning') void refreshPill()
      if (next === 'idle' || next === 'stopped') {
        bubbleThinking.value = ''
        bubbleSays.value = ''
      }
    }),
    await listen<StepStartedPayload>('step_started', (event) => {
      showStep(event.payload)
      onSays(event.payload.text)
    }),
    await listen<ThinkingPayload>('thinking', (event) => onThinking(event.payload)),
    await listen<{ summary: string }>('task_done', (event) => onSays(event.payload.summary)),
    await listen<ObserveStartedPayload>('observe_started', (event) => showObserve(event.payload)),
    await listen<TypedPayload>('typed', (event) => showTyped(event.payload)),
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
    Object.assign(window, {
      __demoStep: showStep,
      __demoObserve: showObserve,
      __demoTyped: showTyped,
      __demoBubble: (thinking: string, says: string) => {
        bubbleThinking.value = thinking
        bubbleSays.value = says
        pill.value = {
          x: window.screenX + window.innerWidth - 80,
          y: window.screenY + window.innerHeight - 80,
          width: 64,
          height: 64,
        }
      },
    })
  }
})
useEventListener(window, 'resize', layout)
onBeforeUnmount(() => unlisten.forEach((off) => off()))
</script>

<template>
  <div class="fixed inset-0">
    <div
      class="absolute inset-0 transition-opacity duration-500 ease-out"
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

    <!-- The speech bubble at the orb's head. Outside the fading frame, so it outlasts it briefly. -->
    <Transition name="bubble">
      <div
        v-if="bubble"
        class="absolute w-[280px] rounded-2xl bg-black/85 px-4 py-2 text-white"
        :style="bubbleStyle"
      >
        <div class="flex items-center gap-2">
          <span class="text-caption text-white/50 uppercase">{{ bubble.label }}</span>
          <span v-if="bubble.live" class="flex gap-1" aria-hidden="true">
            <span
              v-for="n in 3"
              :key="n"
              class="think-dot size-1 rounded-full bg-white"
              :style="{ animationDelay: `${n * 160}ms` }"
            />
          </span>
        </div>
        <p class="line-clamp-3 text-label text-white/90">{{ bubble.text }}</p>
        <span class="absolute right-6 -bottom-1 size-2 rotate-45 bg-black/85" />
      </div>
    </Transition>
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

.bubble-enter-active,
.bubble-leave-active {
  transition:
    opacity 200ms ease,
    transform 200ms ease;
}

.bubble-enter-from,
.bubble-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

.think-dot {
  animation: think-pulse 1s ease-in-out infinite;
}

@keyframes think-pulse {
  0%,
  100% {
    opacity: 0.25;
  }
  50% {
    opacity: 1;
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

  .caret,
  .think-dot {
    animation: none;
  }
}
</style>