<script setup lang="ts">
import { useIntervalFn, useNow, usePreferredReducedMotion } from '@vueuse/core'
import { computed, nextTick, ref, watch, type ComponentPublicInstance } from 'vue'
import type { IndependentTask, TaskBoardModel, TaskPhase } from '@/composables/useTaskBoard'
import { clock } from '@/config/flight'
import { clockText } from '@/config/eta'
import {
  currentStep,
  estimateRemaining,
  isTaskActive,
  taskElapsed,
  taskPosition,
  visibleGoal,
} from '@/config/taskProgress'
import AppIcon from './AppIcon.vue'
const props = defineProps<{ board: TaskBoardModel }>()
const now = useNow({ scheduler: (update) => useIntervalFn(update, 1000) })
const reduceMotion = usePreferredReducedMotion()
const request = computed(() => visibleGoal(props.board.requests.value))
const tasks = computed(() =>
  props.board.tasks.value.filter((task) => task.requestId === request.value?.id),
)
const title = computed(() => request.value?.goals[0]?.title || request.value?.text || '')
const queued = computed(
  () =>
    props.board.requests.value.filter((r) => r.phase === 'queued' && r.id !== request.value?.id)
      .length,
)
const eta = computed(() => estimateRemaining(tasks.value, now.value.getTime()))
const etaText = computed(() => {
  if (!request.value) return ''
  if (!isTaskActive(request.value.phase))
    return request.value.phase === 'done' ? 'Completed' : 'Stopped'
  if (!tasks.value.length || eta.value === null) return 'Estimating…'
  return eta.value > 0 ? `ETA ~${clockText(eta.value)}` : 'Updating estimate…'
})
const expanded = ref(new Set<number>())
const collapsed = ref(new Set<number>())
function isExpanded(task: IndependentTask) {
  return expanded.value.has(task.id) || (task.phase === 'running' && !collapsed.value.has(task.id))
}
function toggle(task: IndependentTask) {
  if (isExpanded(task)) {
    expanded.value.delete(task.id)
    collapsed.value.add(task.id)
  } else {
    collapsed.value.delete(task.id)
    expanded.value.add(task.id)
  }
}
const carousel = ref<HTMLElement | null>(null)
const lanes = new Map<number, HTMLElement>()
const focused = ref<number | null>(null)
function laneRef(id: number, element: Element | ComponentPublicInstance | null) {
  if (element instanceof HTMLElement) lanes.set(id, element)
  else lanes.delete(id)
}
function focusTask(id: number) {
  const element = lanes.get(id)
  if (!element || !carousel.value) return
  focused.value = id
  carousel.value.scrollTo?.({
    left: element.offsetLeft,
    top: 0,
    behavior: reduceMotion.value === 'reduce' ? 'auto' : 'smooth',
  })
}
const position = computed(() =>
  Math.max(
    0,
    tasks.value.findIndex((task) => task.id === focused.value),
  ),
)
function navigate(direction: number) {
  const task =
    tasks.value[Math.max(0, Math.min(tasks.value.length - 1, position.value + direction))]
  if (task) focusTask(task.id)
}
let previousRunning = new Set<number>()
watch(
  () =>
    `${request.value?.id}:${tasks.value
      .filter((t) => t.phase === 'running')
      .map((t) => t.id)
      .join(',')}`,
  async () => {
    const running = tasks.value.filter((t) => t.phase === 'running').map((t) => t.id)
    const target =
      running.find((id) => !previousRunning.has(id)) ??
      (running.includes(focused.value ?? -1) ? undefined : running[0])
    previousRunning = new Set(running)
    if (target !== undefined) {
      await nextTick()
      focusTask(target)
    }
  },
  { immediate: true, flush: 'post' },
)
function onScroll() {
  if (!carousel.value) return
  const left = carousel.value.scrollLeft
  let closest = Infinity
  for (const task of tasks.value) {
    const element = lanes.get(task.id)
    if (element && Math.abs(element.offsetLeft - left) < closest) {
      closest = Math.abs(element.offsetLeft - left)
      focused.value = task.id
    }
  }
}
function status(task: IndependentTask) {
  return {
    queued: task.kind === 'desktop' ? 'Waiting for desktop' : 'Ready',
    planning: 'Planning',
    running: 'Running',
    done: 'Completed',
    failed: 'Failed',
    cancelled: 'Cancelled',
    blocked: 'Blocked',
  }[task.phase]
}
function stepLabel(phase: TaskPhase) {
  return {
    queued: 'Next',
    planning: 'Planning',
    running: 'Now',
    done: 'Done',
    failed: 'Failed',
    cancelled: 'Cancelled',
    blocked: 'Blocked',
  }[phase]
}
</script>
<template>
  <section
    v-if="request"
    aria-label="Tasks Board"
    class="flex flex-col gap-rune-2 rounded-rune-panel border border-rune-outline bg-rune-surface p-rune-2 font-rune-reading text-zinc-100"
    :data-request-id="request.id"
  >
    <header class="flex items-center justify-between gap-2">
      <h2 class="m-0 font-sans text-rune-heading text-zinc-300 uppercase">Tasks Board</h2>
      <span
        data-testid="goal-eta"
        class="text-rune-caption text-zinc-400 tabular-nums"
        title="Estimated from completed steps"
        >{{ etaText }}</span
      >
      <button
        v-if="!isTaskActive(request.phase)"
        type="button"
        class="grid size-7 shrink-0 cursor-pointer place-items-center rounded-full text-zinc-400 hover:bg-white/8 hover:text-white"
        :aria-label="`Dismiss request ${request.id}`"
        @click="board.dismiss(request.id)"
      >
        <AppIcon name="close" size="sm" />
      </button>
    </header>
    <p
      v-if="tasks.length !== 1"
      data-testid="goal-title"
      class="m-0 text-rune-body leading-relaxed [overflow-wrap:anywhere]"
    >
      {{ title }}
    </p>
    <div
      v-if="tasks.length > 1"
      class="flex items-center justify-between gap-2 text-rune-caption text-zinc-500"
    >
      <span data-testid="task-count">Tasks {{ taskPosition(tasks) }}</span>
      <span v-if="tasks.length > 1"
        >{{ tasks.filter((t) => t.phase === 'done').length }} completed</span
      >
    </div>
    <p
      v-if="request.phase === 'planning' || request.phase === 'queued'"
      role="status"
      class="m-0 flex items-center gap-2 text-rune-body text-zinc-400"
    >
      <span class="activity-dots" aria-hidden="true">···</span
      >{{ request.phase === 'planning' ? 'Planning tasks and steps…' : 'Waiting to start…' }}
    </p>
    <p v-if="request.error" role="alert" class="m-0 text-rune-body text-error">
      {{ request.error }}
    </p>
    <div
      v-if="tasks.length > 1"
      class="flex items-center justify-between gap-2 text-rune-caption text-zinc-500"
    >
      <span>Parallel tasks · view {{ position + 1 }}/{{ tasks.length }}</span>
      <div class="flex gap-1">
        <button
          type="button"
          aria-label="Previous task"
          :disabled="position === 0"
          class="grid size-8 cursor-pointer place-items-center rounded-full hover:bg-white/8 disabled:opacity-25"
          @click="navigate(-1)"
        >
          <AppIcon name="chevron" class="rotate-90" size="sm" />
        </button>
        <button
          type="button"
          aria-label="Next task"
          :disabled="position >= tasks.length - 1"
          class="grid size-8 cursor-pointer place-items-center rounded-full hover:bg-white/8 disabled:opacity-25"
          @click="navigate(1)"
        >
          <AppIcon name="chevron" class="-rotate-90" size="sm" />
        </button>
      </div>
    </div>
    <div
      v-if="tasks.length"
      ref="carousel"
      data-testid="task-carousel"
      class="task-carousel"
      :class="{ 'parallel-tasks': tasks.length > 1 }"
      @scroll.passive="onScroll"
    >
      <article
        v-for="(task, index) in tasks"
        :key="task.id"
        :ref="(el) => laneRef(task.id, el)"
        :data-task-id="task.id"
        class="task-lane"
      >
        <button
          type="button"
          :aria-label="`${isExpanded(task) ? 'Collapse' : 'Expand'} task ${index + 1}`"
          :aria-expanded="isExpanded(task)"
          :aria-controls="`task-steps-${task.id}`"
          class="w-full cursor-pointer text-left outline-none focus-visible:ring-1 focus-visible:ring-white/50"
          @click="toggle(task)"
        >
          <span
            v-if="tasks.length > 1"
            class="mb-2 flex items-center justify-between gap-2 text-rune-caption text-zinc-500"
            ><span>TASK {{ index + 1 }}</span
            ><span role="status" class="flex items-center gap-1.5"
              ><span
                class="path-dot"
                :class="{
                  'lit-dot': task.phase === 'running' || task.phase === 'done',
                  'attention-dot': task.phase === 'failed' || task.phase === 'blocked',
                }"
                aria-hidden="true"
              />{{ status(task) }}</span
            ></span
          >
          <span class="block text-rune-body leading-relaxed [overflow-wrap:anywhere]">{{
            task.text
          }}</span>
        </button>
        <div
          class="mt-2 flex items-center justify-between gap-3 text-rune-caption text-zinc-400 tabular-nums"
        >
          <span data-testid="step-count">Step {{ currentStep(task) }}/{{ task.steps.length }}</span>
          <span class="flex items-center gap-1.5"
            ><span class="text-zinc-600">Elapsed</span
            ><time data-testid="task-elapsed">{{
              clock(taskElapsed(task, now.getTime()))
            }}</time></span
          >
        </div>
        <div
          v-if="isExpanded(task)"
          :id="`task-steps-${task.id}`"
          data-testid="task-detail"
          class="mt-rune-2"
        >
          <ol class="step-path m-0 list-none p-0">
            <li
              v-for="(step, stepIndex) in task.steps"
              :key="stepIndex"
              :aria-current="step.phase === 'running' ? 'step' : undefined"
              class="step-node"
              :class="{ 'current-step': step.phase === 'running' }"
            >
              <span class="step-dot" aria-hidden="true" /><span
                class="min-w-0 text-rune-body leading-relaxed [overflow-wrap:anywhere]"
                >{{ stepIndex + 1 }}. {{ step.text
                }}<span class="mt-0.5 block text-rune-caption text-zinc-500">{{
                  stepLabel(step.phase)
                }}</span></span
              >
            </li>
          </ol>
        </div>
        <button
          v-if="isTaskActive(task.phase)"
          type="button"
          class="mt-2 min-h-8 cursor-pointer text-rune-caption text-zinc-500 hover:text-white"
          :aria-label="`Cancel task ${task.id}`"
          @click="board.cancel(task.id)"
        >
          Cancel task
        </button>
      </article>
    </div>
    <footer
      v-if="queued || tasks.length > 1"
      class="flex items-center justify-between gap-2 border-t border-dotted border-white/10 pt-2 text-rune-caption text-zinc-500"
    >
      <span v-if="queued || tasks.length > 1">{{
        queued
          ? `${queued} request${queued === 1 ? '' : 's'} queued`
          : tasks.length > 1
            ? 'Steps run in sequence within each task'
            : 'Steps run in sequence'
      }}</span>
    </footer>
  </section>
</template>
<style scoped>
.task-carousel {
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: 16px;
  max-height: 320px;
  overflow: auto;
  scroll-snap-type: x proximity;
  padding: 0 0 8px;
}
.task-carousel::-webkit-scrollbar {
  height: 0;
}
.parallel-tasks {
  border-top: 1px dotted rgb(255 255 255 / 0.25);
  padding-top: 16px;
}
.task-lane {
  flex: 1 0 min(100%, 220px);
  min-width: 0;
  max-width: 320px;
  scroll-snap-align: start;
}
.path-dot {
  width: 5px;
  height: 5px;
  border: 1px solid rgb(255 255 255 / 0.4);
  border-radius: 50%;
  flex-shrink: 0;
}
.lit-dot {
  background: var(--color-primary);
  border-color: var(--color-primary);
}
.attention-dot {
  background: var(--color-attention);
  border-color: var(--color-attention);
}
.step-path {
  display: flex;
  flex-direction: column;
}
.step-node {
  position: relative;
  display: flex;
  gap: 8px;
  padding-bottom: 16px;
  color: #a1a1aa;
}
.step-node:not(:last-child)::after {
  content: '';
  position: absolute;
  left: 2px;
  top: 10px;
  bottom: 3px;
  border-left: 1px dotted rgb(255 255 255 / 0.2);
}
.step-dot {
  margin-top: 6px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #52525b;
  flex-shrink: 0;
}
.current-step {
  color: var(--color-primary);
}
.current-step .step-dot {
  background: var(--color-primary);
}
.activity-dots {
  letter-spacing: 4px;
  animation: breathe 1.4s ease-in-out infinite;
}
@keyframes breathe {
  50% {
    opacity: 0.3;
  }
}
@media (prefers-reduced-motion: reduce) {
  .activity-dots {
    animation: none;
  }
}
</style>