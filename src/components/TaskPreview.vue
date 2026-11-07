<script setup lang="ts">
import { useIntersectionObserver } from '@vueuse/core'
import { ref, useTemplateRef, watch } from 'vue'
import type { PreviewRun } from '@/config/preview'
import { usePreview } from '@/composables/usePreview'
import AppIcon from './AppIcon.vue'
const props = defineProps<{ run?: PreviewRun | null; fill?: boolean }>()
defineEmits<{ open: [] }>()
const target = useTemplateRef<HTMLElement>('target')
const visible = ref(false)
const picture = ref<string | null>(null)
const preview = usePreview()
let generation = 0
useIntersectionObserver(target, ([entry]) => {
  visible.value = !!entry?.isIntersecting
})
watch(
  [visible, () => props.run?.id, () => props.run?.thumb, () => props.run?.thumbRun],
  async () => {
    const token = ++generation
    picture.value = null
    if (!visible.value || !props.run?.thumb) return
    const url = await preview.fileOf(props.run.thumb, props.run.thumbRun ?? props.run.id)
    if (token === generation) picture.value = url
  },
  { immediate: true },
)
</script>
<template>
  <button
    ref="target"
    type="button"
    :disabled="!run"
    :aria-label="run ? `Preview task: ${run.task}` : 'No task preview available'"
    class="group overflow-hidden bg-black disabled:cursor-default"
    :class="
      fill ? 'absolute inset-0 size-full' : 'relative aspect-video w-24 shrink-0 rounded-rune-tile'
    "
    @click="$emit('open')"
  >
    <img
      v-if="picture"
      :src="picture"
      alt=""
      class="size-full"
      :class="fill ? 'object-cover' : 'object-contain'"
      @error="picture = null"
    />
    <span v-else class="absolute inset-0 grid place-items-center text-rune-text-3"
      ><AppIcon :name="run ? 'monitor' : 'target'"
    /></span>
    <span
      v-if="run"
      class="absolute inset-0 grid place-items-center bg-black/30 text-white opacity-0 transition group-hover:opacity-100 group-focus-visible:opacity-100"
      ><AppIcon name="play"
    /></span>
  </button>
</template>