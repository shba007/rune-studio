<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useEventListener, useIntervalFn, usePreferredReducedMotion, useRafFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'

import { KIND_ICON, TOOL_LABEL, kindOf, type ActionKind } from '@/config/actions'
import { DOCK_WIDTH, ORB_SIZE, PAGE_GAP, PAGE_WIDTH } from '@/config/layout'
import { OVERLAY_MODE } from '@/types/protocol'
import type {
  AgentState,
  CaptureFeedback,
  ScreenRecording,
  ObserveStartedPayload,
  OverlayInfo,
  OverlayOwner,
  PillRect,
  Settings,
  StepStartedPayload,
  ThinkingPayload,
  TypedPayload,
} from '@/types/protocol'

import { useAnalysis } from '@/composables/useAnalysis'
import { useExternalAgent } from '@/composables/useExternalAgent'
import { useModelActivity } from '@/composables/useModelActivity'
import { friendly } from '@/config/errors'

import AgentMark from './AgentMark.vue'
import AnalysisBar from './AnalysisBar.vue'
import AnalysisLayer from './AnalysisLayer.vue'
import OverlayBarShell from './OverlayBarShell.vue'
import AppIcon from './AppIcon.vue'
import CursorGlyph from './CursorGlyph.vue'

const STEP = 8
const DEPTH = 4
const DOT = 1.15
const DROP_INTERVAL = 2.4

const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
const active = ref(false)
const recording = ref(false)
const standaloneRecording = ref<ScreenRecording | null>(null)
const stopHotkey = ref('')
const reducedMotion = usePreferredReducedMotion()
let width = 0
let height = 0
let startedAt = 0
let dots: { x: number; y: number; depth: number; along: number }[] = []

const hotkeyLabel = computed(() => stopHotkey.value.replace(/\+/g, ' + '))

function layout() {
  const el = canvas.value
  if (!el) return
  width = window.innerWidth
  height = window.innerHeight
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
  const reveal = still ? DEPTH : Math.min(DEPTH, seconds * 14)
  const drop = Math.floor(seconds / DROP_INTERVAL)
  const speed = Math.hypot(width, height) / 3
  const origins = [drop - 1, drop]
    .filter((index) => index >= 0)
    .map((index) => ({
      x: width / 2,
      y: height / 2,
      age: seconds - index * DROP_INTERVAL,
    }))

  for (const frameDot of owner.value === 'control' ? dots : []) {
    if (frameDot.depth > reveal) continue
    const edge = 1 - frameDot.depth / DEPTH
    let level = 0.1 + 0.34 * edge * edge
    if (!still) {
      for (const origin of origins) {
        const distance = Math.hypot(frameDot.x - origin.x, frameDot.y - origin.y)
        const wave = Math.exp(-Math.pow((distance - origin.age * speed) / 48, 2))
        level = Math.max(level, wave * (0.65 + 0.35 * edge))
      }
    }
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
  updateCursorEffects(now)
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

interface Ripple {
  x: number
  y: number
  at: number
  pulses: number
  typing?: boolean
}
const RIPPLE_SECONDS = 0.9
let ripples: Ripple[] = []

const cursorEffects = ref<Ripple[]>([])
function updateCursorEffects(now: number) {
  ripples = ripples.filter(
    (ripple) => (now - ripple.at) / 1000 < RIPPLE_SECONDS * ripple.pulses + 0.4,
  )
  cursorEffects.value = ripples.filter((ripple) => now >= ripple.at)
}

interface Flash {
  x: number
  y: number
  w: number
  h: number
  at: number
  continuous: boolean
}
const WAVE_SECONDS = 1.8
const WAVE_BAND = 0.09
const WAVE_PITCH = 2 * STEP
let flashes: Flash[] = []

function drawFlashes(ctx: CanvasRenderingContext2D, now: number, still: boolean) {
  flashes = flashes.filter((flash) => flash.continuous || (now - flash.at) / 1000 < WAVE_SECONDS)
  const visible = [...flashes]
  if (analysisMode.value && analysis.busy.value)
    visible.push({ x: 0, y: 0, w: width, h: height, at: startedAt, continuous: true })
  if (recording.value && overlayInfo.value?.primary && !isRecording())
    visible.push({ x: 0, y: 0, w: width, h: height, at: startedAt, continuous: true })
  const target =
    standaloneRecording.value?.phase === 'recording' ? standaloneRecording.value.monitor : null
  const info = overlayInfo.value
  if (target && info) {
    const scale = info.scaleFactor || 1
    visible.push({
      x: (target.x - info.x) / scale,
      y: (target.y - info.y) / scale,
      w: target.width / scale,
      h: target.height / scale,
      at: startedAt,
      continuous: true,
    })
  }
  for (const flash of visible) {
    const cycle = (now - flash.at) / 1000 / WAVE_SECONDS
    const t = still ? 0.5 : flash.continuous ? cycle % 1 : cycle
    if (t < 0) continue
    const reach = Math.hypot(flash.w / 2, flash.h / 2)
    const front = t * (1 + WAVE_BAND)
    for (let y = flash.y; y <= flash.y + flash.h; y += WAVE_PITCH) {
      for (let x = flash.x; x <= flash.x + flash.w; x += WAVE_PITCH) {
        const radius =
          Math.hypot(x - flash.x - flash.w / 2, y - flash.y - flash.h / 2) / Math.max(1, reach)
        const lit = Math.max(0, 1 - Math.abs(radius - front) / WAVE_BAND)
        if (lit > 0) dot(ctx, x, y, 1.2 + 1.4 * lit, 0.9 * lit)
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
    window.setTimeout(() => {
      if (!active.value) pause()
    }, 600)
  }
}

const agentName = ref<string | null>(null)
const { identity, darkAccent, mark, logo, mono, letter, hint } = useExternalAgent()
const markPulse = ref(false)
watch(identity, (who, before) => {
  if (who && who.id !== before?.id) {
    markPulse.value = true
    setTimeout(() => (markPulse.value = false), 600)
  }
})
const owner = ref<OverlayOwner>('none')
let captureUntil = 0
let captureTimer: number | undefined
let seenCapture = 0
const isRecording = () => standaloneRecording.value?.phase === 'recording'
watch(owner, (next) =>
  setActive(next !== 'none' || performance.now() < captureUntil || isRecording()),
)
watch(standaloneRecording, () =>
  setActive(owner.value !== 'none' || performance.now() < captureUntil || isRecording()),
)
const overlayInfo = ref<OverlayInfo | null>(null)
const captureFeedback = ref<CaptureFeedback | null>(null)
watch([captureFeedback, overlayInfo], () => {
  const feedback = captureFeedback.value
  const info = overlayInfo.value
  if (!feedback || !info || feedback.serial <= seenCapture || feedback.remainingMs <= 0) return
  seenCapture = feedback.serial
  const scale = info.scaleFactor || 1
  const now = performance.now()
  flashes.push({
    x: (feedback.monitor.x - info.x) / scale,
    y: (feedback.monitor.y - info.y) / scale,
    w: feedback.monitor.width / scale,
    h: feedback.monitor.height / scale,
    at: now - (WAVE_SECONDS * 1000 - feedback.remainingMs),
    continuous: false,
  })
  captureUntil = now + feedback.remainingMs
  setActive(true)
  window.clearTimeout(captureTimer)
  captureTimer = window.setTimeout(
    () => setActive(owner.value !== 'none' || isRecording()),
    feedback.remainingMs,
  )
})
const analysis = useAnalysis()
const analysisMode = computed(() => owner.value === 'analysis')
const view = computed(() => ({
  width: overlayInfo.value?.width ?? window.innerWidth * (window.devicePixelRatio || 1),
  height: overlayInfo.value?.height ?? window.innerHeight * (window.devicePixelRatio || 1),
  scaleFactor: overlayInfo.value?.scaleFactor ?? (window.devicePixelRatio || 1),
  originX: overlayInfo.value?.x ?? 0,
  originY: overlayInfo.value?.y ?? 0,
}))
const drawsMarkers = computed(
  () =>
    analysisMode.value &&
    (analysis.snapshot.value.id.startsWith('insight-') ||
      (overlayInfo.value?.primary === true && analysis.snapshot.value.monitor?.primary === true)),
)
const barShown = ref(false)
const startedAtMs = ref(0)
const nowMs = ref(0)
const scanKey = ref(0)
let barTimer: number | undefined
let tick: number | undefined
watch(
  () => analysis.snapshot.value.status,
  (status, before) => {
    window.clearTimeout(barTimer)
    if (status === 'capturing' || (status === 'asking' && before !== 'capturing')) {
      startedAtMs.value = performance.now()
      barShown.value = true
    }
    if (status === 'asking' && before !== 'asking') {
      scanKey.value += 1
      const monitor = analysis.snapshot.value.monitor
      if (analysis.snapshot.value.id.startsWith('insight-') && monitor) {
        const dpr = window.devicePixelRatio || 1
        flashes.push({
          x: monitor.x / dpr - window.screenX,
          y: monitor.y / dpr - window.screenY,
          w: monitor.width / dpr,
          h: monitor.height / dpr,
          at: performance.now(),
          continuous: false,
        })
      }
    }
    if (status === 'done' || status === 'error' || status === 'cancelled') {
      barTimer = window.setTimeout(() => (barShown.value = false), 1500)
    }
    if (status === 'idle') barShown.value = false
  },
)
const elapsedMs = computed(() => Math.max(0, nowMs.value - startedAtMs.value))
const shownName = computed(() => identity.value?.agent ?? agentName.value ?? 'Rune')

const TYPED_TAIL = 24

interface Typing {
  text: string
  startedAt: number
  msPerChar: number
}

const action = ref<{
  key: number
  kind: ActionKind
  label: string
  detail: string
  typing: Typing | null
} | null>(null)

const typedCount = ref(0)
const typedShown = computed(() => {
  const text = action.value?.typing?.text.slice(0, typedCount.value).replaceAll('\n', '↵') ?? ''
  return text.length > TYPED_TAIL ? `…${text.slice(-TYPED_TAIL)}` : text
})

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
    const dpr = window.devicePixelRatio || 1
    ripples.push({
      x: step.point.x / dpr - window.screenX,
      y: step.point.y / dpr - window.screenY,
      at: performance.now() + step.durationMs,
      pulses: step.double ? 2 : 1,
    })
  }
}

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

function showObserve({ region, continuous }: ObserveStartedPayload) {
  typedCount.value = 0
  action.value = {
    key: -++observeCount,
    kind: 'screen',
    label: region ? 'Zoom' : (TOOL_LABEL.observe ?? 'Observe'),
    detail: '',
    typing: null,
  }
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
  if (continuous) flashes = flashes.filter((flash) => !flash.continuous)
  flashes.push({ ...box, at: performance.now(), continuous: Boolean(continuous) })
}

const BUBBLE_CHARS = 120
const bubbleThinking = ref('')
const bubbleSays = ref('')
const bubbleModel = ref('')
const configuredSettings = ref<Settings | null>(null)
const modelActivity = useModelActivity()
const pill = ref<PillRect | null>(null)
let refreshingPill = false
let geometryRevision = 0

async function refreshPill() {
  if (refreshingPill) return
  refreshingPill = true
  const revision = geometryRevision
  try {
    const rect = await invoke<PillRect | null>('pill_rect')
    if (revision === geometryRevision) pill.value = rect
  } catch {
    if (revision === geometryRevision) pill.value = null
  } finally {
    refreshingPill = false
  }
}
useIntervalFn(() => {
  if (bubbleThinking.value || bubbleSays.value) void refreshPill()
}, 100)

const bubble = computed(() => {
  if (!pill.value) return null
  if (bubbleThinking.value) {
    const text = bubbleThinking.value.replace(/\s+/g, ' ').trim()
    const shown = text.length > BUBBLE_CHARS ? `…${text.slice(-BUBBLE_CHARS)}` : text
    return { label: bubbleModel.value || 'Rune', live: true, text: shown }
  }
  if (bubbleSays.value) {
    return { label: bubbleModel.value || 'Rune', live: false, text: bubbleSays.value.trim() }
  }
  return null
})

const bubbleStyle = computed(() => {
  const rect = pill.value
  if (!rect) return {}
  const dpr = window.devicePixelRatio || 1
  const pillScale = rect.scaleFactor ?? dpr
  const pageWidth = Math.min(
    (PAGE_WIDTH + PAGE_GAP) * pillScale,
    Math.max(0, rect.width - (ORB_SIZE + DOCK_WIDTH) * pillScale),
  )
  const originX = overlayInfo.value?.x ?? window.screenX * dpr
  const originY = overlayInfo.value?.y ?? window.screenY * dpr
  const right = (rect.x + rect.width - originX - pageWidth) / dpr
  const top = (rect.y + rect.height - originY - ORB_SIZE * pillScale) / dpr
  return {
    right: `${Math.max(8, window.innerWidth - right)}px`,
    bottom: `${Math.max(8, window.innerHeight - top + 12)}px`,
  }
})

function onThinking({ text, done, model }: ThinkingPayload) {
  bubbleThinking.value = done ? '' : text
  if (model) bubbleModel.value = `${model} · requested`
  void refreshPill()
}

function onSays(text: string, model?: string) {
  bubbleThinking.value = ''
  bubbleModel.value =
    model || (configuredSettings.value?.externalAgent ? agentName.value : null) || 'Rune'
  bubbleSays.value = /^(?:error[:\s]+)?(?:the )?model server answered \d{3}/i.test(text)
    ? friendly(text)
    : text
  void refreshPill()
}

function configuredModelLabel() {
  const settings = configuredSettings.value
  const model = settings?.externalAgent ? undefined : settings?.models?.agent
  return modelActivity.models.value.length > 1
    ? 'Multiple models'
    : model
      ? `${model} · configured`
      : undefined
}

const unlisten: UnlistenFn[] = []

onMounted(async () => {
  layout()
  draw(performance.now())
  unlisten.push(
    await listen<PillRect | null>('pill_geometry', ({ payload }) => {
      geometryRevision++
      pill.value = payload
    }),
    await listen<CaptureFeedback>('capture_feedback', (event) => {
      captureFeedback.value = event.payload
    }),
    await listen<ScreenRecording>('screen_recording', (event) => {
      standaloneRecording.value = event.payload
    }),
  )
  try {
    recording.value = await invoke<boolean>('video_recording_state')
  } catch {}
  unlisten.push(
    await listen<boolean>('video_recording', (event) => {
      recording.value = event.payload
    }),
  )
  try {
    configuredSettings.value = await invoke<Settings>('get_settings')
    stopHotkey.value = configuredSettings.value.stopHotkey
  } catch {}
  unlisten.push(
    await listen('settings_changed', async () => {
      try {
        configuredSettings.value = await invoke<Settings>('get_settings')
      } catch {}
    }),
    await listen<{ state: AgentState }>('state_changed', (event) => {
      const next = event.payload.state
      setActive(
        (owner.value === 'analysis' && analysis.busy.value) ||
          next === 'acting' ||
          next === 'observing' ||
          next === 'planning' ||
          next === 'loading_model' ||
          next === 'awaiting_confirm' ||
          performance.now() < captureUntil ||
          isRecording(),
      )
      if (next !== 'acting' && next !== 'observing' && next !== 'planning') {
        action.value = null
      }
      if (next === 'planning') void refreshPill()
      if (next === 'idle' || next === 'stopped') {
        bubbleThinking.value = ''
      }
    }),
    await listen<StepStartedPayload>('step_started', (event) => {
      showStep(event.payload)
      onSays(event.payload.text, configuredModelLabel())
    }),
    await listen<ThinkingPayload>('thinking', (event) => onThinking(event.payload)),
    await listen<{ text: string }>('confirm_request', (event) => {
      action.value = {
        key: -(1000 + ++observeCount),
        kind: 'wait',
        label: 'Needs your OK',
        detail: event.payload.text,
        typing: null,
      }
    }),
    await listen<{ summary: string; success?: boolean }>('task_done', (event) =>
      onSays(
        event.payload.summary,
        event.payload.success === false ? undefined : configuredModelLabel(),
      ),
    ),
    await listen<string | null>('task_answer', ({ payload }) => {
      if (payload) {
        onSays(payload, configuredModelLabel())
      } else {
        bubbleSays.value = ''
        bubbleThinking.value = ''
      }
    }),
    await listen<ObserveStartedPayload>('observe_started', (event) => showObserve(event.payload)),
    await listen<TypedPayload>('typed', (event) => showTyped(event.payload)),
    await listen('observe_stopped', () => (flashes = flashes.filter((flash) => !flash.continuous))),
    await listen<{ agent: string | null }>(
      'external_agent',
      (event) => (agentName.value = event.payload.agent),
    ),
  )
  unlisten.push(
    await listen<{ mode: OverlayOwner }>(OVERLAY_MODE, (event) => {
      owner.value = event.payload.mode
    }),
  )
  try {
    overlayInfo.value = await invoke<OverlayInfo>('overlay_info')
    if (overlayInfo.value.answer) onSays(overlayInfo.value.answer, configuredModelLabel())
    owner.value = overlayInfo.value.mode
    if (overlayInfo.value.recording) standaloneRecording.value = overlayInfo.value.recording
    const pending = overlayInfo.value.captureFeedback
    if (pending && pending.serial > (captureFeedback.value?.serial ?? 0))
      captureFeedback.value = pending
  } catch {}
  nowMs.value = performance.now()
  tick = window.setInterval(() => (nowMs.value = performance.now()), 250)
  setActive(true)
  try {
    agentName.value = await invoke<string | null>('get_control_label')
  } catch {}
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
onBeforeUnmount(() => {
  unlisten.forEach((off) => off())
  window.clearInterval(tick)
  window.clearTimeout(barTimer)
  window.clearTimeout(captureTimer)
})
</script>

<template>
  <div class="fixed inset-0">
    <CursorGlyph
      v-for="effect in cursorEffects"
      :key="effect.at"
      class="absolute"
      :style="{
        left: `${effect.x - (effect.typing ? 16 : 1)}px`,
        top: `${effect.y - (effect.typing ? 16 : 1)}px`,
      }"
      :kind="effect.typing ? 'text' : 'default'"
      ripple
    />
    <div
      class="absolute inset-0 transition-opacity duration-500 ease-out"
      :class="active ? 'opacity-100' : 'opacity-0'"
    >
      <canvas ref="canvas" class="absolute inset-0 size-full" aria-hidden="true" />

      <OverlayBarShell v-if="owner === 'control'">
        <span
          class="grid shrink-0 place-items-center"
          :class="{ 'motion-safe:animate-pulse': markPulse }"
          :style="{ color: identity ? (darkAccent ?? '#f1f1ef') : '#f1f1ef' }"
          :title="hint"
        >
          <AgentMark
            :glyph="identity ? mark : null"
            :logo="identity ? logo : null"
            :mono="mono"
            :letter="identity ? letter : null"
            :own="!identity"
          />
        </span>
        <span class="text-caption whitespace-nowrap uppercase" :title="hint"
          >{{ shownName }} is in control</span
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

        <div class="flex items-center gap-2 border-l border-white/20 pr-6 pl-3">
          <span v-if="hotkeyLabel" class="text-caption whitespace-nowrap text-white/50 uppercase">{{
            hotkeyLabel
          }}</span>
        </div>
      </OverlayBarShell>
      <OverlayBarShell v-else-if="isRecording()">
        <span class="size-2 rounded-full bg-[#2d71c3] motion-safe:animate-pulse" />
        <span class="text-caption whitespace-nowrap uppercase"
          >Screen recording · {{ standaloneRecording?.seconds }}s</span
        >
        <span class="border-l border-white/20 pr-6 pl-3 text-caption text-white/50 uppercase">{{
          hotkeyLabel
        }}</span>
      </OverlayBarShell>
    </div>

    <template v-if="analysisMode && overlayInfo?.primary">
      <AnalysisBar
        v-if="barShown"
        :status="analysis.snapshot.value.status"
        :elapsed-ms="elapsedMs"
        :hotkey="hotkeyLabel"
      />
    </template>
    <AnalysisLayer
      v-if="drawsMarkers"
      :snapshot="analysis.snapshot.value"
      :view="view"
      :scan-key="scanKey"
    />

    <Transition name="bubble">
      <div
        v-if="bubble"
        class="absolute w-[280px] rounded-2xl bg-black/85 px-4 py-2 text-white"
        :style="bubbleStyle"
      >
        <div class="flex items-center gap-2">
          <span class="min-w-0 break-all text-caption text-white/50 uppercase">{{
            bubble.label
          }}</span>
          <span v-if="bubble.live" class="flex gap-1" aria-hidden="true">
            <span
              v-for="n in 3"
              :key="n"
              class="think-dot size-1 rounded-full bg-white"
              :style="{ animationDelay: `${n * 160}ms` }"
            />
          </span>
        </div>
        <p
          class="m-0 text-label whitespace-pre-wrap text-white/90 [overflow-wrap:anywhere]"
          :class="bubble.live ? 'line-clamp-3' : ''"
        >
          {{ bubble.text }}
        </p>
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