<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, usePreferredReducedMotion, useRafFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'

import type { AgentState, Settings, StepStartedPayload } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import type { IconName } from '@/icons'

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

/** The one caption under the control pill: what the agent is doing right now. */
/** Name of the outside agent in control, or null when it is Rune Studio's own agent. */
const agentName = ref<string | null>(null)

/** Characters of typed text shown at once; longer text scrolls so the newest letters stay in view. */
const TYPED_TAIL = 24

interface Typing {
  text: string
  startedAt: number
  msPerChar: number
}

const action = ref<{
  step: number
  icon: IconName
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

const ACTION_ICON: Record<string, IconName> = {
  click: 'click',
  type_text: 'keyboard',
  press_keys: 'shortcut',
  open_app: 'app',
  wait: 'wait',
}
const ACTION_LABEL: Record<string, string> = {
  click: 'Click',
  type_text: 'Type',
  press_keys: 'Keys',
  open_app: 'Open',
  wait: 'Wait',
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
    step: step.step,
    typing,
    icon: ACTION_ICON[step.tool] ?? 'click',
    label: prefix ? `${prefix}click` : (ACTION_LABEL[step.tool] ?? step.tool),
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
  // Dev only: lets a plain browser preview play a step without the Rust side.
  if (import.meta.env.DEV)
    (window as unknown as { __demoStep: typeof showStep }).__demoStep = showStep
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
      <span class="text-caption uppercase">{{ agentName ?? 'Rune' }} is in control</span>
      <span v-if="hotkeyLabel" class="text-caption text-white/50 uppercase"
        >· Stop {{ hotkeyLabel }}</span
      >
    </div>

    <!-- What the agent is doing right now. One caption at a time, replaced by the next. -->
    <div class="absolute top-16 left-1/2 -translate-x-1/2">
      <Transition name="action" mode="out-in">
        <div
          v-if="action"
          :key="action.step"
          class="flex h-8 items-center gap-2 rounded-full bg-white/95 px-4 text-[#111] shadow-[0_0_0_1px_rgb(0_0_0/0.25)]"
        >
          <AppIcon :name="action.icon" size="sm" />
          <span class="text-caption uppercase">{{ action.label }}</span>
          <span v-if="action.detail" class="max-w-64 truncate text-label text-[#111]/70">{{
            action.detail
          }}</span>
          <span
            v-if="action.typing"
            class="flex items-center text-label whitespace-pre text-[#111]/70"
          >
            {{ typedShown }}<span class="caret ml-px h-4 w-[2px] bg-[#111]" />
          </span>
        </div>
      </Transition>
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