<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useIntervalFn, useNow } from '@vueuse/core'
import type { LiveStepsModel } from '@/composables/useLiveSteps'
import {
  STEP_KIND,
  stepKey,
  stepDuration,
  durationText,
  stepStatus,
  stepTitle,
  type LiveStep,
} from '@/config/liveSteps'
import RunLanes from './RunLanes.vue'
import WorkRow from './WorkRow.vue'
import AppIcon from './AppIcon.vue'
import ToolResultView from './ToolResultView.vue'
const props = defineProps<{
  live: LiveStepsModel
  focus?: { key: string; serial: number } | null
}>()
const request = computed(() =>
  props.live.snapshot.value?.request?.phase === 'queued'
    ? null
    : props.live.snapshot.value?.request,
)
const tasks = computed(() => props.live.snapshot.value?.tasks ?? [])
const steps = computed(() =>
  props.live.steps.value
    .filter((step) => tasks.value.some((task) => task.id === step.taskId))
    .slice()
    .sort((a, b) => (a.startedAt ?? 0) - (b.startedAt ?? 0)),
)
const picked = ref<string | null>(props.focus?.key ?? null)
watch(
  () => props.focus,
  (value) => {
    if (value) picked.value = value.key
  },
)
watch(
  () => request.value?.id,
  (id, old) => {
    if (old != null && id !== old) picked.value = null
  },
)
const selected = computed(
  () => steps.value.find((step) => stepKey(step) === picked.value) ?? steps.value.at(-1) ?? null,
)
const now = useNow({ scheduler: (update) => useIntervalFn(update, 1000) })
const elapsed = computed(() =>
  request.value?.startedAt == null
    ? '0 s'
    : durationText(
        Math.max(0, (request.value.finishedAt ?? now.value.getTime()) - request.value.startedAt),
      ),
)
const named = (step: LiveStep) =>
  stepTitle(
    step.kind === 'tool' && step.detail.includes('__') ? { ...step, text: step.detail } : step,
  )
const server = computed(() => (selected.value ? named(selected.value).server : null))
const short = (step: LiveStep) => named(step).title
const timing = (step: LiveStep) => {
  const ms = stepDuration(step, now.value.getTime())
  return ms === null ? '' : durationText(ms)
}
</script>

<template>
  <section
    v-if="request"
    aria-label="Current run"
    class="flex min-w-0 flex-col gap-4 rounded-rune-panel bg-rune-surface p-4"
  >
    <header class="flex flex-wrap items-center gap-3">
      <div class="min-w-0 flex-1">
        <h2 class="m-0 text-rune-heading text-rune-text-2 uppercase">Current run</h2>
        <p class="m-0 text-rune-body break-words">{{ request.text }}</p>
      </div>
      <span class="text-rune-caption text-rune-text-2">{{ request.phase }} · {{ elapsed }}</span>
    </header>
    <p
      v-if="live.snapshot.value?.droppedResults || live.snapshot.value?.droppedSteps"
      role="status"
      class="m-0 text-rune-caption text-rune-text-2"
    >
      {{ live.snapshot.value?.droppedResults }} older results and
      {{ live.snapshot.value?.droppedSteps }} older steps omitted from this view.
    </p>
    <RunLanes
      :lanes="tasks.map((task) => ({ id: task.id, label: task.text }))"
      :steps="steps"
      :now="now.getTime()"
      :show-detail="false"
      :selected-key="selected ? stepKey(selected) : null"
      @select="picked = $event"
    />
    <div class="grid min-w-0 gap-4 md:grid-cols-[minmax(180px,1fr)_minmax(0,2fr)]">
      <div class="max-h-[36rem] min-w-0 overflow-y-auto" aria-label="Current run steps">
        <section v-for="task in tasks" :key="task.id" class="mb-3">
          <h3 class="m-0 px-2 py-2 text-rune-caption text-rune-text-2">{{ task.text }}</h3>
          <WorkRow
            class="[&>div>button]:min-w-0"
            v-for="step in steps.filter((item) => item.taskId === task.id)"
            :key="stepKey(step)"
            :title="short(step)"
            :meta="[timing(step), stepStatus(step)].filter(Boolean).join(' · ')"
            interactive
            :selected="!!selected && stepKey(selected) === stepKey(step)"
            :data-step-key="stepKey(step)"
            @open="picked = stepKey(step)"
          >
            <template #leading
              ><AppIcon
                :name="STEP_KIND[step.kind].icon"
                size="sm"
                :class="step.kind === 'tool' ? 'text-tool' : 'text-voice'"
            /></template>
          </WorkRow>
        </section>
      </div>
      <article
        v-if="selected"
        class="min-w-0"
        aria-label="Selected live step"
        :data-selected-step="stepKey(selected)"
      >
        <h3 class="m-0 text-rune-body">{{ selected.text || selected.tool }}</h3>
        <dl class="my-3 grid grid-cols-2 gap-2 text-rune-caption">
          <div>
            <dt class="text-rune-text-2">Kind</dt>
            <dd class="m-0">{{ STEP_KIND[selected.kind].label }}</dd>
          </div>
          <div v-if="server">
            <dt class="text-rune-text-2">Server</dt>
            <dd class="m-0 break-words">{{ server }}</dd>
          </div>
          <div v-if="selected.model">
            <dt class="text-rune-text-2">Model</dt>
            <dd class="m-0 break-words">{{ selected.model }}</dd>
          </div>
          <div>
            <dt class="text-rune-text-2">Timing</dt>
            <dd class="m-0">{{ timing(selected) }} · {{ stepStatus(selected) }}</dd>
          </div>
          <div v-if="selected.startedAt != null">
            <dt class="text-rune-text-2">Started</dt>
            <dd class="m-0">{{ new Date(selected.startedAt).toLocaleTimeString() }}</dd>
          </div>
          <div v-if="selected.finishedAt != null">
            <dt class="text-rune-text-2">Finished</dt>
            <dd class="m-0">{{ new Date(selected.finishedAt).toLocaleTimeString() }}</dd>
          </div>
        </dl>
        <p class="text-rune-body whitespace-pre-wrap break-words">{{ selected.detail }}</p>
        <p
          v-if="selected.ok === false"
          role="alert"
          class="text-rune-body text-attention whitespace-pre-wrap break-words"
        >
          {{ selected.error || 'Step failed' }}
        </p>
        <div v-else-if="selected.kind === 'tool' && selected.result" aria-label="Live tool result">
          <p
            v-if="selected.result.structuredContentOmitted"
            class="text-rune-caption text-rune-text-2"
          >
            Structured result omitted for size.
          </p>
          <ToolResultView
            :text="selected.result.text"
            :schema="selected.result.outputSchema"
            :structured-content="selected.result.structuredContent"
            :show-raw="false"
          />
        </div>
      </article>
      <p v-else class="text-rune-body text-rune-text-2">Steps appear here as they start.</p>
    </div>
  </section>
</template>