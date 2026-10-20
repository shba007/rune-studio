<script setup lang="ts">
import { computed } from 'vue'

/**
 * The orb's outer ring, as the progress of the task: one large dotted arc per goal. Finished goals
 * are lit, the current one pulses, the rest are dim, and a small arrow marks the goal being worked
 * on. While the plan is still being written the arrow circles an empty ring.
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
  /** An outside agent's colour for the lit arcs (on the dark rim); null is the usual light grey. */
  accent?: string | null
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

const planning = computed(() => props.goals === 0 && props.running)
const visible = computed(() => props.goals > 0 || props.running)

/** The arrow sits in the middle of the goal being worked on, and at the end once all are done. */
const arrowTurn = computed(() => {
  if (props.goals <= 0) return '0deg'
  if (props.finished) return `${360 - 360 / props.goals / 2}deg`
  return `${((props.current + 0.5) * 360) / props.goals}deg`
})
</script>

<template>
  <svg
    v-if="visible"
    :viewBox="`0 0 ${SIZE} ${SIZE}`"
    class="ring"
    :style="{ '--lit': accent ?? '#f1f1ef' }"
    aria-hidden="true"
  >
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
    <!-- Planning: a dim ring; the arrow below circles it. -->
    <path v-else :d="arc(0, 359.9)" fill="none" class="dots pending" />

    <g
      class="arrow-turn"
      :class="{ planning }"
      :style="{ transform: planning ? undefined : `rotate(${arrowTurn})` }"
    >
      <polygon
        points="35.4,0.7 30.6,-1.6 30.6,3"
        class="arrow"
        :class="{ failed: failed && !finished }"
      />
    </g>
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
  stroke: var(--lit, #f1f1ef);
}

.pending {
  stroke: rgb(241 241 239 / 0.2);
}

.current {
  stroke: var(--lit, #f1f1ef);
  animation: pulse 1.4s ease-in-out infinite;
}

.current.failed {
  stroke: #d71921;
  animation: none;
}

.arrow-turn {
  transform-origin: 32px 32px;
  transition: transform 600ms cubic-bezier(0.3, 0, 0.2, 1);
}

.arrow {
  fill: var(--lit, #f1f1ef);
}

.arrow.failed {
  fill: #d71921;
}

.planning {
  animation: spin 2.2s linear infinite;
  transition: none;
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
  .planning {
    animation: none;
  }
}
</style>