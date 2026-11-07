<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import RuneIconButton from './RuneIconButton.vue'
import { useIntervalFn, useNow, usePreferredReducedMotion } from '@vueuse/core'
import { computed, nextTick, ref, watch, type ComponentPublicInstance } from 'vue'
import type { IndependentTask, TaskBoardModel, TaskPhase } from '@/composables/useTaskBoard'
import { clock } from '@/config/preview'
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
import RuneLoader from './RuneLoader.vue'
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
  return (
    expanded.value.has(task.id) ||
    (['running', 'planning'].includes(task.phase) && !collapsed.value.has(task.id))
  )
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
    aria-label="Tasks"
    class="flex flex-col gap-rune-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-ink"
    :data-request-id="request.id"
  >
    <header class="flex items-center justify-between gap-2">
      <h2 class="m-0 font-sans text-rune-heading text-rune-text-2 uppercase">Tasks</h2>
      <span
        data-testid="goal-eta"
        class="text-rune-caption text-rune-text-2 tabular-nums"
        title="Estimated from completed steps"
        >{{ etaText }}</span
      >
      <RuneIconButton
        variant="ghost"
        size="sm"
        v-if="!isTaskActive(request.phase)"
        :label="`Dismiss request ${request.id}`"
        @click="board.dismiss(request.id)"
      >
        <AppIcon name="close" size="sm" />
      </RuneIconButton>
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
      class="flex items-center justify-between gap-2 text-rune-caption text-rune-text-2"
    >
      <span data-testid="task-count">Tasks {{ taskPosition(tasks) }}</span>
      <span v-if="tasks.length > 1"
        >{{ tasks.filter((t) => t.phase === 'done').length }} completed</span
      >
    </div>
    <p
      v-if="request.phase === 'planning' || request.phase === 'queued'"
      role="status"
      class="m-0 flex items-center gap-2 text-rune-body text-rune-text-2"
    >
      <RuneLoader />
      {{ request.phase === 'planning' ? 'Planning tasks and steps…' : 'Waiting to start…' }}
    </p>
    <p v-if="request.error" role="alert" class="m-0 text-rune-body text-error">
      {{ request.error }}
    </p>
    <div
      v-if="tasks.length > 1"
      class="flex items-center justify-between gap-2 text-rune-caption text-rune-text-2"
    >
      <span>Parallel tasks · view {{ position + 1 }}/{{ tasks.length }}</span>
      <div class="flex gap-1">
        <RuneIconButton
          variant="ghost"
          size="sm"
          label="Previous task"
          :disabled="position === 0"
          @click="navigate(-1)"
        >
          <AppIcon name="chevron" class="rotate-90" size="sm" />
        </RuneIconButton>
        <RuneIconButton
          variant="ghost"
          size="sm"
          label="Next task"
          :disabled="position >= tasks.length - 1"
          @click="navigate(1)"
        >
          <AppIcon name="chevron" class="-rotate-90" size="sm" />
        </RuneIconButton>
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
          class="w-full cursor-pointer text-left"
          @click="toggle(task)"
        >
          <span
            v-if="tasks.length > 1"
            class="mb-2 flex items-center justify-between gap-2 text-rune-caption text-rune-text-2"
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
          class="mt-2 flex items-center justify-between gap-3 text-rune-caption text-rune-text-2 tabular-nums"
        >
          <span data-testid="step-count">Step {{ currentStep(task) }}/{{ task.steps.length }}</span>
          <span class="flex items-center gap-1.5"
            ><span class="text-rune-text-3">Elapsed</span
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
          <p
            v-if="task.phase === 'planning' && !task.steps.length"
            role="status"
            class="m-0 flex items-center gap-2 text-rune-caption text-rune-text-2"
          >
            <RuneLoader /> Planning steps…
          </p>
          <ol class="step-path m-0 list-none p-0">
            <li
              v-for="(step, stepIndex) in task.steps"
              :key="stepIndex"
              :aria-current="step.phase === 'running' ? 'step' : undefined"
              class="step-node"
              :class="{
                'current-step': step.phase === 'running',
                'finished-step': step.phase === 'done',
                'problem-step': step.phase === 'failed' || step.phase === 'blocked',
              }"
            >
              <RuneLoader v-if="step.phase === 'planning'" class="step-loader" />
              <AppIcon v-else-if="step.phase === 'done'" name="ok" size="sm" class="step-icon" />
              <span v-else class="step-dot" aria-hidden="true" />
              <span class="min-w-0 text-rune-body leading-relaxed [overflow-wrap:anywhere]">
                {{ step.text }}
                <span class="step-status">{{ stepLabel(step.phase) }}</span>
              </span>
            </li>
          </ol>
        </div>
        <RuneButton
          variant="ghost"
          size="sm"
          v-if="isTaskActive(task.phase)"
          class="mt-2"
          :aria-label="`Cancel task ${task.id}`"
          @click="board.cancel(task.id)"
        >
          Cancel task
        </RuneButton>
      </article>
    </div>
    <footer
      v-if="queued || tasks.length > 1"
      class="flex items-center justify-between gap-2 pt-2 text-rune-caption text-rune-text-2"
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
  display: flex;
  gap: 8px;
  align-items: flex-start;
  padding-bottom: 6px;
  color: var(--color-rune-text-2);
}
.step-dot {
  margin: 4px 1px 0;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--color-rune-text-3);
  opacity: 0.35;
  flex-shrink: 0;
}
.step-icon {
  margin-top: 3px;
  flex-shrink: 0;
  color: var(--color-rune-ink);
}
.step-loader {
  width: 14px;
  margin: 4px 0 0 1px;
}
.current-step {
  color: var(--color-rune-ink);
}
.current-step .step-dot {
  opacity: 1;
  background: var(--color-rune-text-2);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-rune-text-2) 25%, transparent);
}
.problem-step .step-dot {
  background: var(--color-attention);
  opacity: 1;
}
.step-status {
  display: block;
  font-size: 0.75em;
  color: var(--color-rune-text-2);
}
</style>