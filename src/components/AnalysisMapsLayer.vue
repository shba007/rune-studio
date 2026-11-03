<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue'
import { colorClassMap } from '@/config/segmentation'
import type { OverlayView } from '@/config/analysis'
import type { AnalysisSnapshot } from '@/types/protocol'
import VisualSpecializedLayer from './VisualSpecializedLayer.vue'

const props = defineProps<{ snapshot: AnalysisSnapshot; view: OverlayView }>()
const canvas = useTemplateRef<HTMLCanvasElement>('canvas')
const ready = ref(false)
const style = computed(() => {
  const capture = props.snapshot.monitor
  if (!capture) return {}
  const scale = props.view.scaleFactor || 1
  return {
    left: `${(capture.x - (props.view.originX ?? 0)) / scale}px`,
    top: `${(capture.y - (props.view.originY ?? 0)) / scale}px`,
    width: `${capture.width / scale}px`,
    height: `${capture.height / scale}px`,
  }
})
watch(
  () => props.snapshot.visualMaps,
  (maps, _, onCleanup) => {
    let cancelled = false
    onCleanup(() => {
      cancelled = true
    })
    ready.value = false
    if (!maps?.segmentation) return
    const map = new Image()
    map.onload = () => {
      if (cancelled || !canvas.value) return
      const context = canvas.value.getContext('2d', { willReadFrequently: true })
      if (!context) return
      try {
        canvas.value.width = map.naturalWidth
        canvas.value.height = map.naturalHeight
        context.drawImage(map, 0, 0)
        const pixels = context.getImageData(0, 0, canvas.value.width, canvas.value.height)
        colorClassMap(pixels.data, new Set(maps.classes.map((item) => item.id)))
        context.putImageData(pixels, 0, 0)
        ready.value = true
      } catch {
        ready.value = false
      }
    }
    map.src = maps.segmentation
  },
  { immediate: true, flush: 'post' },
)
</script>

<template>
  <div
    v-if="snapshot.monitor && !snapshot.stale && snapshot.status === 'done'"
    class="pointer-events-none absolute overflow-hidden"
    :style="style"
    aria-hidden="true"
  >
    <canvas
      v-show="ready && snapshot.visibleGroups?.regions !== false"
      ref="canvas"
      class="absolute inset-0 size-full opacity-30"
    />
    <img
      v-if="snapshot.visualMaps?.depth && snapshot.visibleGroups?.depth !== false"
      :src="snapshot.visualMaps.depth"
      alt=""
      class="absolute inset-0 size-full opacity-20"
    />
    <VisualSpecializedLayer
      v-if="snapshot.visibleGroups?.entities !== false"
      :instances="snapshot.visualMaps?.instances ?? []"
      :oriented="snapshot.visualMaps?.oriented ?? []"
    />
  </div>
</template>