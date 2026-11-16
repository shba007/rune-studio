<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  STEP_KIND,
  durationText,
  laneLayout,
  stepDuration,
  stepKey,
  stepStatus,
  type LiveStep,
} from '@/config/liveSteps'
import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'

const props = defineProps<{
  lanes: { id: number | null; label: string }[]
  steps: LiveStep[]
  now: number
}>()

const laneIds = computed(() => new Set(props.lanes.map((lane) => lane.id)))
const shown = computed(() => props.steps.filter((step) => laneIds.value.has(step.taskId)))
const layout = computed(() => laneLayout(shown.value, props.now))
const kinds = computed(() =>
  (Object.keys(STEP_KIND) as LiveStep['kind'][]).filter((kind) =>
    shown.value.some((step) => step.kind === kind),
  ),
)

const picked = ref<string | null>(null)
const selected = computed(
  () =>
    shown.value.find((step) => stepKey(step) === picked.value) ??
    shown.value.filter((step) => step.ok === null).at(-1) ??
    shown.value.at(-1) ??
    null,
)
const selectedDuration = computed(() => {
  const ms = selected.value ? stepDuration(selected.value, props.now) : null
  return ms === null ? '' : durationText(ms)
})

function describe(step: LiveStep) {
  const ms = stepDuration(step, props.now)
  return [
    STEP_KIND[step.kind].label,
    step.text || step.tool,
    ms === null ? '' : durationText(ms),
    stepStatus(step),
  ]
    .filter(Boolean)
    .join(' · ')
}
</script>

<template>
  <section v-if="layout.blocks.length" aria-label="Run timeline" class="flex flex-col gap-1.5">
    <div
      v-for="lane in lanes"
      :key="lane.id ?? 'run'"
      class="lane"
      :class="{ 'lane-labelled': lanes.length > 1 }"
      :data-lane="lane.id ?? 'run'"
    >
      <span v-if="lanes.length > 1" class="truncate text-rune-caption text-rune-text-2">{{
        lane.label
      }}</span>
      <div class="track">
        <template v-for="block in layout.blocks" :key="stepKey(block.step)">
          <RuneButton
            v-if="block.step.taskId === lane.id"
            variant="link"
            class="block-mark"
            :class="{
              'block-running': block.step.ok === null,
              'block-failed': block.step.ok === false,
              'block-picked': selected && stepKey(selected) === stepKey(block.step),
            }"
            :data-kind="block.step.kind"
            :style="{ left: `${block.left}%`, width: `${block.width}%` }"
            :aria-label="describe(block.step)"
            :aria-pressed="!!selected && stepKey(selected) === stepKey(block.step)"
            :title="describe(block.step)"
            @click="picked = stepKey(block.step)"
          />
        </template>
      </div>
    </div>

    <div class="flex items-center gap-3 text-rune-caption text-rune-text-3">
      <span v-for="kind in kinds" :key="kind" class="flex items-center gap-1">
        <span class="legend-mark" :data-kind="kind" aria-hidden="true" />{{ STEP_KIND[kind].label }}
      </span>
      <span class="ml-auto tabular-nums" data-testid="lane-span">{{
        durationText(layout.span)
      }}</span>
    </div>

    <div
      v-if="selected"
      data-testid="lane-detail"
      class="flex flex-col gap-1 rounded-rune-tile bg-rune-sunken p-2"
    >
      <div class="flex min-w-0 items-center gap-1.5 text-rune-caption text-rune-ink">
        <AppIcon
          :name="STEP_KIND[selected.kind].icon"
          size="sm"
          class="shrink-0"
          :class="selected.kind === 'tool' ? 'text-tool' : 'text-voice'"
        />
        <span class="min-w-0 flex-1 truncate" :title="selected.text || selected.tool">{{
          selected.text || selected.tool
        }}</span>
        <span
          class="shrink-0 tabular-nums"
          :class="selected.ok === false ? 'text-attention' : 'text-rune-text-2'"
          >{{ selected.ok === false ? 'Failed' : selectedDuration || stepStatus(selected) }}</span
        >
      </div>
      <div class="flex min-w-0 flex-wrap gap-x-2 gap-y-0.5 text-rune-caption text-rune-text-3">
        <span>{{ STEP_KIND[selected.kind].label }}</span>
        <span v-if="selected.model" class="max-w-full truncate" :title="selected.model">{{
          selected.model
        }}</span>
        <span>{{ stepStatus(selected) }}</span>
      </div>
      <p
        v-if="selected.detail && selected.detail !== (selected.text || selected.tool)"
        class="m-0 line-clamp-2 text-rune-caption text-rune-text-2 [overflow-wrap:anywhere]"
        :title="selected.detail"
      >
        {{ selected.detail }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.lane {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  align-items: center;
  gap: 8px;
}
.lane-labelled {
  grid-template-columns: 44px minmax(0, 1fr);
}
.track {
  position: relative;
  height: 18px;
  border-radius: 5px;
  background: var(--color-rune-sunken);
  overflow: hidden;
}
.block-mark {
  position: absolute;
  top: 3px;
  height: 12px;
  min-width: 5px;
  padding: 0;
  border-radius: 3px;
  background: var(--color-voice);
  transition: width 1s linear;
}
.block-mark[data-kind='tool'],
.legend-mark[data-kind='tool'] {
  background: var(--color-tool);
}
.block-failed {
  background: var(--color-attention) !important;
}
.block-running {
  animation: block-pulse 1.4s ease-in-out infinite;
}
.block-picked {
  box-shadow: 0 0 0 1.5px var(--color-rune-ink);
  z-index: 1;
}
.legend-mark {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  background: var(--color-voice);
}
@keyframes block-pulse {
  50% {
    opacity: 0.5;
  }
}
@media (prefers-reduced-motion: reduce) {
  .block-mark {
    transition: none;
  }
  .block-running {
    animation: none;
  }
}
</style>