<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'
import { readImageAsPng } from '@/config/imageFile'

defineProps<{ id: string; label: string; disabled?: boolean }>()
const image = defineModel<{ name: string; data: string } | null>({ required: true })
const notice = ref('')
let revision = 0

async function choose(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  const current = ++revision
  image.value = null
  notice.value = ''
  try {
    const data = await readImageAsPng(file)
    if (current === revision) image.value = { name: file.name, data }
  } catch (error) {
    if (current === revision) notice.value = error instanceof Error ? error.message : String(error)
  }
}
onBeforeUnmount(() => {
  revision++
})
</script>

<template>
  <div class="flex flex-col gap-2 text-rune-body">
    <span class="text-rune-heading text-rune-text-3 uppercase">Image</span>
    <label
      :for="id"
      class="grid min-h-28 place-items-center overflow-hidden rounded-rune-tile border border-dashed border-rune-text-3 bg-rune-surface p-3 text-center text-rune-caption text-rune-text-2"
      :class="disabled ? 'cursor-default opacity-40' : 'cursor-pointer'"
    >
      <img
        v-if="image"
        :src="image.data"
        :alt="image.name"
        class="max-h-40 max-w-full rounded-rune-tile object-contain"
      />
      <span v-else>Choose an image</span>
    </label>
    <input
      :id="id"
      type="file"
      accept="image/*"
      :aria-label="label"
      :disabled="disabled"
      class="sr-only"
      @change="choose"
    />
    <p v-if="image" class="m-0 truncate text-rune-caption" :title="image.name">{{ image.name }}</p>
    <p v-if="notice" role="alert" class="m-0 text-rune-caption text-error">{{ notice }}</p>
  </div>
</template>