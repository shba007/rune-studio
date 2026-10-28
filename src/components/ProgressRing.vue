<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  goals: number
  current: number
  finished: boolean
  running: boolean
  failed: boolean
  accent?: string | null
  progress?: number | null
}>()

const SIZE = 64
const CENTER = SIZE / 2
const RADIUS = 30
const GAP_DEGREES = 14

type Status = 'done' | 'current' | 'pending'

function point(degrees: number): [number, number] {
  const radians = ((degrees - 90) * Math.PI) / 180
  return [CENTER + RADIUS * Math.cos(radians), CENTER + RADIUS * Math.sin(radians)]
}

function arc(from: number, to: number): string {
  const [x1, y1] = point(from)
  const [x2, y2] = point(to)
  return `M ${x1.toFixed(2)} ${y1.toFixed(2)} A ${RADIUS} ${RADIUS} 0 ${to - from > 180 ? 1 : 0} 1 ${x2.toFixed(2)} ${y2.toFixed(2)}`
}

const segments = computed(() => {
  const n = props.goals
  const slice = 360 / n
  const gap = n === 1 ? 0 : GAP_DEGREES
  return Array.from({ length: n }, (_, index) => {
    const from = index * slice + gap / 2
    const to = from + slice - gap - (n === 1 ? 0.01 : 0)
    const status: Status =
      props.finished || index < props.current
        ? 'done'
        : index === props.current
          ? 'current'
          : 'pending'
    return { d: arc(from, to), status }
  })
})

const visible = computed(() => props.progress != null || props.goals > 0 || props.running)
const completed = computed(() => Math.max(0, Math.min(1, props.progress ?? 0)) * 100)
</script>

<template>
  <svg
    v-if="visible"
    :viewBox="`0 0 ${SIZE} ${SIZE}`"
    class="ring"
    :style="{ '--lit': accent ?? '#2d71c3' }"
    aria-hidden="true"
  >
    <template v-if="progress != null">
      <circle :cx="CENTER" :cy="CENTER" :r="RADIUS" fill="none" class="dots pending" />
      <circle
        v-if="completed > 0"
        :cx="CENTER"
        :cy="CENTER"
        :r="RADIUS"
        fill="none"
        class="dots done task-progress"
        pathLength="100"
        :stroke-dasharray="`${completed} 100`"
      />
    </template>
    <template v-else-if="goals > 0">
      <path
        v-for="(segment, index) in segments"
        :key="index"
        :d="segment.d"
        fill="none"
        class="dots"
        :class="[segment.status, { failed: failed && segment.status === 'current' }]"
      />
    </template>
    <path v-else :d="arc(0, 359.9)" fill="none" class="dots pending planning-ring" />
  </svg>
</template>

<style scoped>
.ring {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.dots {
  stroke-width: 3;
  stroke-linecap: round;
  transition: stroke 300ms ease;
}

.done {
  stroke: var(--lit, #2d71c3);
}
.task-progress {
  transform: rotate(-90deg);
  transform-origin: center;
  transition: stroke-dasharray 300ms ease;
}

.pending {
  stroke: rgb(255 255 255 / 0.16);
}

.current {
  stroke: var(--lit, #2d71c3);
  animation: pulse 1.4s ease-in-out infinite;
}

.current.failed {
  stroke: #d71921;
  animation: none;
}

.planning-ring {
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  50% {
    opacity: 0.35;
  }
}

@media (prefers-reduced-motion: reduce) {
  .current,
  .planning-ring {
    animation: none;
  }
}
</style>