<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { VisualResult } from '@/config/playground'
import { visualSnapshot } from '@/config/visualFindings'
import PlaygroundResult from './PlaygroundResult.vue'
import PlaygroundSegmentation from './PlaygroundSegmentation.vue'
import VisualSpecializedLayer from './VisualSpecializedLayer.vue'

const props = defineProps<{ result: VisualResult; image: string }>()
const revision = ref(0)
watch(
  () => props.result,
  () => revision.value++,
)
const snapshot = computed(() => ({
  ...visualSnapshot(props.result, props.image),
  id: `playground-visual-${revision.value}`,
}))
const instances = computed(() =>
  (props.result.data ?? []).flatMap((item) =>
    item.mask && /^data:image\/png;base64,[a-z0-9+/=]+$/i.test(item.mask)
      ? [{ mask: item.mask, label: item.label }]
      : [],
  ),
)
const oriented = computed(() => {
  const width = props.result.usage?.image_width ?? 0
  const height = props.result.usage?.image_height ?? 0
  if (width <= 0 || height <= 0) return []
  return (props.result.data ?? []).flatMap((item) => {
    const points = item.oriented_box?.corners_pixels
    if (
      !points ||
      points.length !== 4 ||
      !points.every((point) => point.length === 2 && point.every(Number.isFinite))
    )
      return []
    return [
      {
        label: item.label,
        corners: points.map(
          ([x, y]) => [(x / width) * 1000, (y / height) * 1000] as [number, number],
        ),
      },
    ]
  })
})
</script>

<template>
  <PlaygroundResult v-if="result.data" :snapshot="snapshot" :live="false">
    <template #annotations
      ><VisualSpecializedLayer :instances="instances" :oriented="oriented"
    /></template>
  </PlaygroundResult>
  <PlaygroundSegmentation
    v-else-if="result.class_map && result.classes"
    :image="image"
    :class-map="result.class_map"
    :classes="result.classes"
  />
  <p
    v-if="result.data?.some((item) => item.pose?.length)"
    class="m-0 text-center text-rune-caption text-rune-text-3"
  >
    Only keypoints with at least 50% confidence are shown on the image.
  </p>
</template>