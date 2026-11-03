<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'
import { classColor, colorClassMap } from '@/config/segmentation'

const props = defineProps<{
  image: string
  classMap: string
  classes: { id: number; label: string }[]
}>()
const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
const error = ref('')
const opacity = ref(60)
const ready = ref(false)

watch(
  () => [props.classMap, props.classes] as const,
  ([source, classes], _, onCleanup) => {
    let cancelled = false
    onCleanup(() => {
      cancelled = true
    })
    error.value = ''
    ready.value = false
    const map = new Image()
    map.onload = () => {
      if (cancelled) return
      try {
        const target = canvas.value
        const context = target?.getContext('2d', { willReadFrequently: true })
        if (!target || !context || !map.naturalWidth || !map.naturalHeight) throw new Error()
        target.width = map.naturalWidth
        target.height = map.naturalHeight
        context.drawImage(map, 0, 0)
        const pixels = context.getImageData(0, 0, target.width, target.height)
        colorClassMap(pixels.data, new Set(classes.map((item) => item.id)))
        context.putImageData(pixels, 0, 0)
        ready.value = true
      } catch {
        error.value = 'Could not display the segmentation map.'
      }
    }
    map.onerror = () => {
      if (!cancelled) error.value = 'Could not read the segmentation map.'
    }
    map.src = source
  },
  { immediate: true, flush: 'post' },
)

const color = (id: number) => `rgb(${classColor(id).join(' ')})`
</script>

<template>
  <figure class="m-0 flex min-w-0 flex-col items-center gap-4" data-testid="segmentation-result">
    <div class="relative max-w-full overflow-hidden rounded-rune-tile bg-rune-sunken">
      <img :src="image" alt="Source image" class="block max-h-[60vh] max-w-full object-contain" />
      <canvas
        ref="canvas"
        v-show="ready"
        role="img"
        aria-label="Colored semantic segmentation"
        class="pointer-events-none absolute inset-0 size-full"
        :style="{ opacity: opacity / 100 }"
      />
    </div>
    <p v-if="error" role="alert" class="m-0 text-rune-body text-error">{{ error }}</p>
    <label class="flex items-center gap-3 text-rune-caption text-rune-text-2">
      Overlay opacity
      <input
        v-model.number="opacity"
        type="range"
        min="0"
        max="100"
        aria-label="Segmentation overlay opacity"
      />
      {{ opacity }}%
    </label>
    <figcaption
      class="flex max-w-full flex-wrap justify-center gap-x-4 gap-y-2"
      aria-label="Segmentation legend"
    >
      <span
        v-for="item in classes"
        :key="item.id"
        class="inline-flex items-center gap-2 text-rune-body"
      >
        <span
          class="size-3 shrink-0 rounded-rune-tile"
          :style="{ backgroundColor: color(item.id) }"
          aria-hidden="true"
        ></span>
        {{ item.label }} <span class="text-rune-caption text-rune-text-3">({{ item.id }})</span>
      </span>
    </figcaption>
  </figure>
</template>