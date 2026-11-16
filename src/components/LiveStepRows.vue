<script setup lang="ts">
import { useIntervalFn, useNow } from '@vueuse/core'
import {
  STEP_KIND,
  durationText,
  stepDuration,
  stepStatus,
  type LiveStep,
} from '@/config/liveSteps'
import AppIcon from './AppIcon.vue'
defineProps<{ steps: LiveStep[] }>()
const now = useNow({ scheduler: (update) => useIntervalFn(update, 1000) })
function outcome(step: LiveStep) {
  if (step.ok !== true) return stepStatus(step)
  const ms = stepDuration(step, now.value.getTime())
  return ms === null ? 'Done' : durationText(ms)
}
</script>

<template>
  <ol class="m-0 max-h-64 list-none overflow-y-auto p-0" aria-label="Live steps">
    <li
      v-for="(step, index) in steps"
      :key="step.step"
      class="flex min-w-0 items-center gap-1.5 py-1 text-rune-caption"
      :title="`${step.text || step.tool} · ${step.detail}`"
    >
      <span
        class="flex size-3 shrink-0 items-center justify-center"
        :class="step.kind === 'tool' ? 'text-tool' : 'text-voice'"
        role="img"
        :aria-label="step.kind === 'tool' ? 'Tool' : 'Screen'"
      >
        <AppIcon :name="STEP_KIND[step.kind].icon" size="sm" />
      </span>
      <span class="min-w-0 flex-1 truncate text-rune-ink">{{ step.text || step.tool }}</span>
      <span
        v-if="step.model && (index === 0 || step.model !== steps[index - 1]?.model)"
        class="max-w-20 truncate text-rune-text-2"
        :title="step.model"
        >{{ step.model }}</span
      >
      <span
        class="shrink-0 tabular-nums"
        :class="step.ok === false ? 'text-attention' : 'text-rune-text-2'"
        :title="stepStatus(step)"
        >{{ outcome(step) }}</span
      >
    </li>
  </ol>
</template>