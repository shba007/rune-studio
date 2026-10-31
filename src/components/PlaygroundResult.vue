<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { drawable, markerOpacity, readPose, stageWord } from '@/config/analysis'
import { friendly } from '@/config/errors'
import type { AnalysisSnapshot } from '@/types/protocol'

import AnalysisAnswer from './AnalysisAnswer.vue'
import AnalysisFindingRow from './AnalysisFindingRow.vue'
import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneChip from './RuneChip.vue'
import WhenText from './WhenText.vue'

const props = defineProps<{ snapshot: AnalysisSnapshot; live: boolean }>()
const emit = defineEmits<{ cancel: [] }>()

const selected = ref<number | null>(null)
watch(
  () => props.snapshot.id,
  () => (selected.value = null),
)

const busy = computed(() => ['capturing', 'asking'].includes(props.snapshot.status))
const stage = computed(() => stageWord(props.snapshot.status))
const failed = computed(() => props.snapshot.status === 'error')
const markers = computed(() =>
  drawable(props.snapshot).map((finding) => {
    const pose = readPose(finding.pose)
    const points = pose?.keypoints ?? []
    const box = finding.box ?? [
      Math.min(...points.map((p) => p.x)),
      Math.min(...points.map((p) => p.y)),
      Math.max(...points.map((p) => p.x)),
      Math.max(...points.map((p) => p.y)),
    ]
    return {
      n: finding.n,
      label: finding.label,
      box,
      pose,
      opacity: markerOpacity(finding.n, selected.value),
    }
  }),
)

const dots = computed(() => props.snapshot.findings.filter((finding) => finding.point !== null))
const toggle = (n: number) => (selected.value = selected.value === n ? null : n)
const pct = (value: number) => `${value / 10}%`
</script>

<template>
  <div class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-y-auto p-4" data-testid="result">
    <header class="flex min-h-8 items-center gap-2">
      <RuneChip :variant="failed ? 'red' : 'neutral'" data-testid="result-stage">
        {{ stage ?? 'Ready' }}
      </RuneChip>
      <span v-if="snapshot.model" class="min-w-0 truncate text-rune-caption text-rune-text-2">
        {{ snapshot.model }}
      </span>
      <span v-if="snapshot.createdUtc" class="text-rune-caption text-rune-text-3">
        <WhenText :iso="snapshot.createdUtc" />
      </span>
      <RuneButton
        v-if="busy && live"
        size="sm"
        variant="danger"
        class="ml-auto"
        data-testid="result-cancel"
        @click="emit('cancel')"
      >
        <AppIcon name="stop" size="sm" /> Cancel
      </RuneButton>
    </header>

    <p v-if="failed" role="alert" class="m-0 text-rune-body text-error">
      {{ friendly(snapshot.error ?? 'The analysis failed.') }}
    </p>

    <div
      v-if="snapshot.screenshot"
      class="relative mx-auto w-fit max-w-full overflow-hidden rounded-rune-tile"
      data-testid="result-frame"
    >
      <img
        :src="snapshot.screenshot"
        alt="Captured frame"
        class="block max-h-[56vh] max-w-full object-contain"
      />
      <div
        v-for="marker in markers"
        :key="marker.n"
        class="absolute rounded-rune-tile bg-voice/10 outline-2 outline-voice"
        :style="{
          left: pct(marker.box[0]),
          top: pct(marker.box[1]),
          width: pct(marker.box[2] - marker.box[0]),
          height: pct(marker.box[3] - marker.box[1]),
          opacity: marker.opacity,
        }"
        :data-marker="marker.n"
      >
        <span
          class="absolute -top-2 -left-2 grid size-5 place-items-center rounded-full bg-rune-ink text-rune-caption font-bold text-black"
          >{{ marker.n }}</span
        >
      </div>
      <span
        v-for="finding in dots"
        :key="`point-${finding.n}`"
        class="absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-attention outline-2 outline-white"
        :style="{ left: pct(finding.point![0]), top: pct(finding.point![1]) }"
        :data-point="finding.n"
      />
      <template v-for="marker in markers" :key="`pose-${marker.n}`">
        <template v-if="marker.pose">
          <svg
            class="pointer-events-none absolute inset-0 size-full"
            viewBox="0 0 1000 1000"
            preserveAspectRatio="none"
            :style="{ opacity: marker.opacity }"
          >
            <line
              v-for="([a, b], index) in marker.pose.connections"
              :key="index"
              :x1="marker.pose.keypoints[a]!.x"
              :y1="marker.pose.keypoints[a]!.y"
              :x2="marker.pose.keypoints[b]!.x"
              :y2="marker.pose.keypoints[b]!.y"
              stroke="#67e8f9"
              stroke-width="2"
              vector-effect="non-scaling-stroke"
            />
          </svg>
          <span
            v-for="(point, index) in marker.pose.keypoints"
            :key="index"
            class="absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-rune-ink"
            :style="{ left: pct(point.x), top: pct(point.y), opacity: marker.opacity }"
            :data-keypoint="point.label"
          />
        </template>
      </template>
    </div>

    <AnalysisAnswer
      v-if="snapshot.status === 'done' || busy"
      :text="snapshot.answer"
      :waiting="busy"
    />

    <ol
      v-if="snapshot.findings.length && !busy"
      class="m-0 flex list-none flex-col gap-2 p-0"
      aria-label="Findings"
    >
      <AnalysisFindingRow
        v-for="finding in snapshot.findings"
        :key="finding.n"
        :finding="finding"
        :open="selected === finding.n"
        :selected="selected === finding.n"
        @toggle="toggle"
      />
    </ol>
  </div>
</template>