<script setup lang="ts">
import { useElementSize } from '@vueuse/core'
import { computed, useTemplateRef } from 'vue'

import { layoutChips } from '@/config/analysis'
import type { Box, Finding } from '@/types/protocol'

/**
 * The screenshot with its annotations on top. Everything is positioned in 0..1000 units across the
 * picture, so it scales with the picture: boxes with corner brackets and leader lines in an SVG,
 * numbered markers and label chips as buttons above it. Monochrome; red only marks an issue.
 */
const props = defineProps<{
  image: string
  width: number
  height: number
  findings: Finding[]
  active: number | null
  /** Draw the boxes, markers and chips (the `analysisBoxes` setting). */
  showBoxes: boolean
}>()
const emit = defineEmits<{ hover: [n: number | null]; select: [n: number] }>()

const picture = useTemplateRef<HTMLElement>('picture')
const { width: shownWidth, height: shownHeight } = useElementSize(picture)

const located = computed(() =>
  props.showBoxes
    ? props.findings.filter((finding): finding is Finding & { box: Box } => finding.box !== null)
    : [],
)
const chips = computed(() =>
  layoutChips(
    located.value.map((finding) => ({ n: finding.n, label: finding.label, box: finding.box })),
    { width: shownWidth.value, height: shownHeight.value },
  ),
)
const chipFor = (n: number) => chips.value.find((chip) => chip.n === n)

/** The four corner brackets of a box as one path; the arm is a third of the shorter side, at most 28 units. */
function brackets([x0, y0, x1, y1]: Box): string {
  const arm = Math.min(28, (x1 - x0) / 3, (y1 - y0) / 3)
  return (
    `M${x0} ${y0 + arm}V${y0}H${x0 + arm}` +
    `M${x1 - arm} ${y0}H${x1}V${y0 + arm}` +
    `M${x1} ${y1 - arm}V${y1}H${x1 - arm}` +
    `M${x0 + arm} ${y1}H${x0}V${y1 - arm}`
  )
}

const dimmed = (n: number) => props.active !== null && props.active !== n
const aspect = computed(() =>
  props.width > 0 && props.height > 0 ? `${props.width} / ${props.height}` : '16 / 9',
)
</script>

<template>
  <div
    ref="picture"
    class="relative w-full overflow-hidden rounded-lg bg-black/80 shadow-[inset_0_0_0_1px_rgb(128_128_128/0.35)]"
    :style="{ aspectRatio: aspect }"
  >
    <img
      :src="image"
      alt="The screenshot that was analysed"
      class="block size-full select-none"
      draggable="false"
    />

    <svg
      class="pointer-events-none absolute inset-0 size-full"
      viewBox="0 0 1000 1000"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <g
        v-for="finding in located"
        :key="finding.n"
        class="motion-safe:transition-opacity"
        :opacity="dimmed(finding.n) ? 0.35 : 1"
      >
        <rect
          :x="finding.box[0]"
          :y="finding.box[1]"
          :width="finding.box[2] - finding.box[0]"
          :height="finding.box[3] - finding.box[1]"
          fill="#fff"
          :fill-opacity="active === finding.n ? 0.22 : 0.08"
        />
        <path
          :d="brackets(finding.box)"
          fill="none"
          stroke="#000"
          stroke-opacity="0.6"
          stroke-width="4"
          vector-effect="non-scaling-stroke"
        />
        <path
          :d="brackets(finding.box)"
          fill="none"
          :stroke="finding.kind === 'issue' ? '#d71921' : '#fff'"
          stroke-width="2"
          vector-effect="non-scaling-stroke"
        />
        <template v-if="chipFor(finding.n)?.leader">
          <line
            v-for="pass in [0, 1]"
            :key="pass"
            :x1="chipFor(finding.n)!.leader!.x1"
            :y1="chipFor(finding.n)!.leader!.y1"
            :x2="chipFor(finding.n)!.leader!.x2"
            :y2="chipFor(finding.n)!.leader!.y2"
            :stroke="pass === 0 ? '#000' : finding.kind === 'issue' ? '#d71921' : '#fff'"
            :stroke-opacity="pass === 0 ? 0.6 : 1"
            :stroke-width="pass === 0 ? 3 : 1"
            vector-effect="non-scaling-stroke"
          />
        </template>
      </g>
    </svg>

    <template v-for="finding in located" :key="finding.n">
      <button
        type="button"
        class="absolute grid size-5 ring-1 ring-black/50 -translate-x-1/2 -translate-y-1/2 cursor-pointer place-items-center rounded-full text-caption outline-none motion-safe:transition focus-visible:ring-2 focus-visible:ring-white"
        :class="[
          active === finding.n ? 'bg-white text-black' : 'bg-black/80 text-white',
          finding.kind === 'issue' && active !== finding.n ? 'ring-1 ring-[#d71921]' : '',
          dimmed(finding.n) ? 'opacity-60' : '',
        ]"
        :style="{ left: `${finding.box[0] / 10}%`, top: `${finding.box[1] / 10}%` }"
        :aria-label="`Finding ${finding.n}: ${finding.label}`"
        :aria-pressed="active === finding.n"
        @mouseenter="emit('hover', finding.n)"
        @mouseleave="emit('hover', null)"
        @focus="emit('hover', finding.n)"
        @blur="emit('hover', null)"
        @click="emit('select', finding.n)"
      >
        {{ finding.n }}
      </button>
      <button
        v-if="chipFor(finding.n)"
        type="button"
        tabindex="-1"
        class="absolute flex h-[22px] cursor-pointer items-center gap-1 overflow-hidden rounded-sm px-2 text-caption uppercase whitespace-nowrap motion-safe:transition"
        :class="[
          active === finding.n ? 'bg-white text-black' : 'bg-black/80 text-white',
          dimmed(finding.n) ? 'opacity-60' : '',
        ]"
        :style="{
          left: `${chipFor(finding.n)!.x / 10}%`,
          top: `${chipFor(finding.n)!.y / 10}%`,
          maxWidth: `${chipFor(finding.n)!.w / 10}%`,
        }"
        @mouseenter="emit('hover', finding.n)"
        @mouseleave="emit('hover', null)"
        @click="emit('select', finding.n)"
      >
        <span class="opacity-60">{{ String(finding.n).padStart(2, '0') }}</span>
        <span class="truncate">{{ finding.label }}</span>
      </button>
    </template>
  </div>
</template>