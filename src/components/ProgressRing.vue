<script setup lang="ts">
import { computed } from 'vue'

/**
 * The orb's outer ring as a dial: a ring of ticks that fill up as the task's goals are done, a
 * longer tick at every goal boundary, and a small arrow at the leading edge. While the plan is
 * still being written the arrow circles an empty dial.
 */
const props = defineProps<{
  /** Number of goals in the plan (0 while there is none). */
  goals: number
  /** Index of the goal being worked on; equal to `goals` when everything is done. */
  current: number
  finished: boolean
  /** A task is running, so a dial with no plan yet means "planning". */
  running: boolean
  failed: boolean
}>()

const SIZE = 64
const CENTER = SIZE / 2
const TICKS = 60
const INNER = 28.6
const OUTER = 31
const LONG_INNER = 28
const LONG_OUTER = 32

/** How far round the dial the task is (0..1). The goal being worked on counts a little. */
const progress = computed(() => {
  if (props.goals <= 0) return 0
  if (props.finished) return 1
  return Math.min(1, (props.current + 0.4) / props.goals)
})

/** Tick numbers that mark where one goal ends and the next begins. */
const boundaries = computed(() => {
  const marks = new Set<number>([0])
  for (let goal = 1; goal < props.goals; goal++) marks.add(Math.round((goal * TICKS) / props.goals))
  return marks
})

const ticks = computed(() =>
  Array.from({ length: TICKS }, (_, index) => {
    const angle = (index / TICKS) * Math.PI * 2 - Math.PI / 2
    const long = boundaries.value.has(index)
    const [from, to] = long ? [LONG_INNER, LONG_OUTER] : [INNER, OUTER]
    return {
      x1: CENTER + from * Math.cos(angle),
      y1: CENTER + from * Math.sin(angle),
      x2: CENTER + to * Math.cos(angle),
      y2: CENTER + to * Math.sin(angle),
      lit: props.goals > 0 && (index + 0.5) / TICKS <= progress.value,
      long,
    }
  }),
)

const planning = computed(() => props.goals === 0 && props.running)
const visible = computed(() => props.goals > 0 || props.running)
const arrowTurn = computed(() => `${progress.value * 360}deg`)
</script>

<template>
  <svg v-if="visible" :viewBox="`0 0 ${SIZE} ${SIZE}`" class="ring" aria-hidden="true">
    <line
      v-for="(tick, index) in ticks"
      :key="index"
      :x1="tick.x1"
      :y1="tick.y1"
      :x2="tick.x2"
      :y2="tick.y2"
      class="tick"
      :class="{ lit: tick.lit, long: tick.long }"
    />
    <!-- The arrow rides the leading edge: it turns to the task's progress, or circles while planning. -->
    <g
      class="arrow-turn"
      :class="{ planning }"
      :style="{ transform: planning ? undefined : `rotate(${arrowTurn})` }"
    >
      <polygon
        points="35.4,2.1 30.6,-0.2 30.6,4.4"
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

.tick {
  stroke: rgb(241 241 239 / 0.16);
  stroke-width: 1;
  stroke-linecap: round;
  transition: stroke 300ms ease;
}

.tick.long {
  stroke: rgb(241 241 239 / 0.34);
}

.tick.lit {
  stroke: #f1f1ef;
}

.arrow-turn {
  transform-origin: 32px 32px;
  transition: transform 600ms cubic-bezier(0.3, 0, 0.2, 1);
}

.arrow {
  fill: #f1f1ef;
  animation: arrow-pulse 1.4s ease-in-out infinite;
}

.arrow.failed {
  fill: #d71921;
  animation: none;
}

.planning {
  animation: spin 2.2s linear infinite;
  transition: none;
}

@keyframes arrow-pulse {
  50% {
    opacity: 0.45;
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .arrow,
  .planning {
    animation: none;
  }
}
</style>