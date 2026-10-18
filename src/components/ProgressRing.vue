<script setup lang="ts">
import { computed } from 'vue'

/**
 * The orb's outer ring, as the progress of the task: one dotted arc per goal. Finished goals are
 * lit, the current one pulses, the rest are dim. While the plan is still being written a lit
 * arc circles the whole ring.
 */
const props = defineProps<{
  /** Number of goals in the plan (0 while there is none). */
  goals: number
  /** Index of the goal being worked on; equal to `goals` when everything is done. */
  current: number
  finished: boolean
  /** A task is running, so a ring with no plan yet means "planning". */
  running: boolean
  failed: boolean
}>()

const SIZE = 64
const CENTER = SIZE / 2
const RADIUS = 30
const GAP_DEGREES = 14

type Status = 'done' | 'current' | 'pending'

/** Where a point at `degrees` (0 = top, clockwise) on the ring is. */
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
    // A full circle cannot be one arc, so a single goal stops just short of closing.
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

const visible = computed(() => props.goals > 0 || props.running)
</script>

<template>
  <svg v-if="visible" :viewBox="`0 0 ${SIZE} ${SIZE}`" class="ring" aria-hidden="true">
    <template v-if="goals > 0">
      <path
        v-for="(segment, index) in segments"
        :key="index"
        :d="segment.d"
        fill="none"
        class="dots"
        :class="[segment.status, { failed: failed && segment.status === 'current' }]"
      />
    </template>
    <template v-else>
      <!-- Planning: a dim ring with a lit arc running round it. -->
      <path :d="arc(0, 359.9)" fill="none" class="dots pending" />
      <g class="spin">
        <path :d="arc(0, 70)" fill="none" class="dots done" />
      </g>
    </template>
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

/* Round caps on zero-length dashes make dots, in keeping with the dot-matrix orb. */
.dots {
  stroke-width: 3;
  stroke-linecap: round;
  stroke-dasharray: 0 4.4;
  transition: stroke 300ms ease;
}

.done {
  stroke: #f1f1ef;
}

.pending {
  stroke: rgb(241 241 239 / 0.2);
}

.current {
  stroke: #f1f1ef;
  animation: pulse 1.4s ease-in-out infinite;
}

.current.failed {
  stroke: #d71921;
  animation: none;
}

.spin {
  transform-origin: 32px 32px;
  animation: spin 2.2s linear infinite;
}

@keyframes pulse {
  50% {
    opacity: 0.35;
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .current,
  .spin {
    animation: none;
  }
}
</style>