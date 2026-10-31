<script setup lang="ts">
import { computed, ref } from 'vue'

import AppIcon from './AppIcon.vue'

const props = defineProps<{
  original: string
  output: string | null
  label: string
  width: number
  height: number
}>()

const split = ref(50)
const natural = ref<{ width: number; height: number } | null>(null)
const ratio = computed(() => {
  const size = natural.value ?? props
  return size.width / size.height || 1
})

function onLoad(event: Event) {
  const image = event.target as HTMLImageElement
  natural.value = { width: image.naturalWidth, height: image.naturalHeight }
}
</script>

<template>
  <div
    class="relative overflow-hidden rounded-rune-tile"
    :style="{
      aspectRatio: String(ratio),
      width: `min(100%, calc(60vh * ${ratio}))`,
    }"
    data-testid="compare"
  >
    <img
      :src="original"
      alt="Source frame"
      class="block size-full"
      data-testid="compare-original"
      @load="onLoad"
    />
    <template v-if="output">
      <img
        :src="output"
        :alt="label"
        class="absolute inset-0 size-full"
        :style="{ clipPath: `inset(0 ${100 - split}% 0 0)` }"
        data-testid="compare-output"
      />
      <span
        class="pointer-events-none absolute inset-y-0 w-0.5 -translate-x-1/2 bg-white"
        :style="{ left: `${split}%` }"
      >
        <span
          class="absolute top-1/2 left-1/2 grid size-8 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full bg-white text-black"
        >
          <AppIcon name="convert" size="sm" />
        </span>
      </span>
      <input
        v-model.number="split"
        type="range"
        min="0"
        max="100"
        step="0.5"
        :aria-label="`Compare source and ${label.toLowerCase()}`"
        class="absolute inset-0 size-full cursor-ew-resize opacity-0"
        data-testid="compare-split"
      />
    </template>
  </div>
</template>