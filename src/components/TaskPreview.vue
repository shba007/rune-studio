<script setup lang="ts">
import { useIntersectionObserver } from '@vueuse/core'
import { ref, useTemplateRef, watch } from 'vue'
import type { FlightRun } from '@/config/flight'
import { useFlight } from '@/composables/useFlight'
import AppIcon from './AppIcon.vue'
const props = defineProps<{ run?: FlightRun | null }>()
defineEmits<{ open: [] }>()
const target = useTemplateRef<HTMLElement>('target')
const visible = ref(false)
const picture = ref<string | null>(null)
const flight = useFlight()
let generation = 0
useIntersectionObserver(target, ([entry]) => {
  visible.value = !!entry?.isIntersecting
})
watch(
  [visible, () => props.run?.id, () => props.run?.thumb],
  async () => {
    const token = ++generation
    picture.value = null
    if (!visible.value || !props.run?.thumb) return
    const url = await flight.fileOf(props.run.thumb, props.run.id)
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
    class="group relative aspect-video w-full overflow-hidden rounded-rune-tile bg-black disabled:cursor-default sm:w-44 sm:shrink-0"
    @click="$emit('open')"
  >
    <img
      v-if="picture"
      :src="picture"
      alt=""
      class="size-full object-contain"
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