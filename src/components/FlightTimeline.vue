<script setup lang="ts">
import { useElementSize } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import {
  clock,
  failedStep,
  markerOf,
  parseIso,
  phaseTimes,
  stepAtTime,
  timeAtPixel,
  toolIcon,
  visibleSegments,
  zoomed,
  type FlightEvent,
  type FlightStep,
  type Segment,
} from '@/config/flight'

const props = defineProps<{
  segments: Segment[]
  steps: FlightStep[]
  starts: number[]
  total: number
  timeMs: number
  events: FlightEvent[]
  runStartedUtc: string | null
  retried: ReadonlySet<number>
}>()
const emit = defineEmits<{ seek: [ms: number] }>()

const track = ref<HTMLElement | null>(null)
const { width: viewport } = useElementSize(track)
const zoom = ref(1)
const scrollLeft = ref(0)
const dragging = ref(false)

const basePx = computed(() =>
  props.total > 0 && viewport.value > 0 ? viewport.value / props.total : 0,
)
const pxPerMs = computed(() => basePx.value * zoom.value)
const contentWidth = computed(() => Math.max(viewport.value, props.total * pxPerMs.value))
const current = computed(() => stepAtTime(props.starts, props.timeMs))
const drawn = computed(() =>
  visibleSegments(props.segments, pxPerMs.value, scrollLeft.value, viewport.value),
)

const OPACITY = { observe: 0.35, decide: 0.55, gate: 0.75, act: 1, settle: 0.22 } as const

function bars(index: number, length: number) {
  const phases = phaseTimes(props.steps[index].phases).filter((item) => item.ms > 0)
  const sum = phases.reduce((all, item) => all + item.ms, 0)
  const scale = sum > 0 ? Math.min(1, length / sum) : 0
  return phases.map((item) => ({
    phase: item.phase,
    opacity: OPACITY[item.phase],
    width: `${(item.ms * scale * 100) / Math.max(length, 1)}%`,
  }))
}

const markers = computed(() => {
  const origin = parseIso(props.runStartedUtc)
  if (origin === null || props.starts.length === 0) return []
  const first = props.steps[0]
  const shift = (parseIso(first?.startedUtc) ?? origin) - origin + (first?.phases.observe ?? 0)
  return props.events.flatMap((event) => {
    const mark = markerOf(event)
    const at = parseIso(event.atUtc)
    if (!mark || at === null) return []
    const ms = Math.min(props.total, Math.max(0, at - origin - shift))
    return [{ id: event.id, ms, ...mark }]
  })
})
const shownMarkers = computed(() =>
  markers.value.filter((mark) => {
    const x = mark.ms * pxPerMs.value
    return x >= scrollLeft.value - 60 && x <= scrollLeft.value + viewport.value + 60
  }),
)

function seekAt(clientX: number) {
  const box = track.value?.getBoundingClientRect()
  if (!box) return
  emit('seek', timeAtPixel(clientX - box.left, scrollLeft.value, pxPerMs.value, props.total))
}

function onDown(event: PointerEvent) {
  dragging.value = true
  ;(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId)
  seekAt(event.clientX)
}
function onMove(event: PointerEvent) {
  if (dragging.value) seekAt(event.clientX)
}
function onUp() {
  dragging.value = false
}

function onWheel(event: WheelEvent) {
  const box = track.value?.getBoundingClientRect()
  if (!box || basePx.value === 0) return
  const pointer = event.clientX - box.left
  const anchor = timeAtPixel(pointer, scrollLeft.value, pxPerMs.value, props.total)
  zoom.value = zoomed(zoom.value, event.deltaY)
  setScroll(anchor * pxPerMs.value - pointer)
}

function setScroll(value: number) {
  const max = Math.max(0, props.total * pxPerMs.value - viewport.value)
  scrollLeft.value = Math.min(max, Math.max(0, value))
}

watch(
  () => props.timeMs,
  (ms) => {
    const x = ms * pxPerMs.value
    if (x < scrollLeft.value || x > scrollLeft.value + viewport.value) {
      setScroll(x - viewport.value / 2)
    }
  },
)
watch(viewport, () => setScroll(scrollLeft.value))

const playheadLeft = computed(() => props.timeMs * pxPerMs.value - scrollLeft.value)
</script>

<template>
  <section class="flex flex-col gap-1 px-4 pt-2" aria-label="Timeline">
    <div
      ref="track"
      class="relative h-20 cursor-pointer touch-none overflow-hidden rounded-xl border border-white/10 bg-[#101010] select-none"
      role="slider"
      tabindex="0"
      aria-label="Session timeline"
      aria-orientation="horizontal"
      :aria-valuemin="0"
      :aria-valuemax="Math.round(total / 1000)"
      :aria-valuenow="Math.round(timeMs / 1000)"
      :aria-valuetext="`${clock(timeMs)}, step ${current + 1} of ${steps.length}`"
      data-testid="track"
      @pointerdown="onDown"
      @pointermove="onMove"
      @pointerup="onUp"
      @pointercancel="onUp"
      @wheel.prevent="onWheel"
    >
      <div
        v-for="segment in drawn"
        :key="segment.index"
        class="absolute top-0 bottom-0 border-l border-white/10"
        :class="[
          segment.index === current ? 'bg-white/[0.07]' : '',
          failedStep(steps[segment.index]) ? 'text-[#ff6b72]' : 'text-zinc-200',
        ]"
        :style="{
          left: `${segment.start * pxPerMs - scrollLeft}px`,
          width: `${Math.max(segment.length * pxPerMs, 6)}px`,
        }"
        data-testid="segment"
        :data-step="segment.index"
      >
        <AppIcon
          v-if="segment.length * pxPerMs >= 18"
          :name="toolIcon(steps[segment.index].tool)"
          size="sm"
          class="absolute top-2 left-1.5 opacity-80"
        />
        <span
          v-if="retried.has(segment.index)"
          class="absolute top-0 left-0 h-2 w-px bg-zinc-100"
          title="A model call was retried in this step"
          data-testid="retry-tick"
        />
        <div class="absolute right-0 bottom-3 left-0 flex h-3">
          <span
            v-for="bar in bars(segment.index, segment.length)"
            :key="bar.phase"
            class="h-full"
            :style="{
              width: bar.width,
              opacity: bar.opacity,
              backgroundImage: 'radial-gradient(circle, currentColor 1.1px, transparent 1.6px)',
              backgroundSize: '5px 5px',
            }"
            :data-phase="bar.phase"
          />
        </div>
      </div>
      <span
        v-for="mark in shownMarkers"
        :key="mark.id"
        class="absolute top-1 -translate-x-1/2 rounded-full px-1.5 text-caption"
        :class="mark.error ? 'bg-[#d71921]/25 text-[#ff6b72]' : 'bg-white/10 text-zinc-300'"
        :style="{ left: `${mark.ms * pxPerMs - scrollLeft}px` }"
        data-testid="marker"
        >{{ mark.word }}</span
      >
      <span
        class="pointer-events-none absolute top-0 bottom-0 w-px bg-[#f1f1ef]"
        :style="{ left: `${playheadLeft}px` }"
        data-testid="playhead"
      />
    </div>
  </section>
</template>