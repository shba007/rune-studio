<script setup lang="ts">
import type { FlightRun } from '@/config/flight'
import RuneDot from './RuneDot.vue'
import TaskPreview from './TaskPreview.vue'

withDefaults(
  defineProps<{
    title: string
    meta?: string
    state?: 'idle' | 'ok' | 'warn' | 'down' | 'busy'
    run?: FlightRun | null
    progress?: number | null
    selected?: boolean
  }>(),
  { selected: false, progress: null },
)
defineEmits<{ open: [] }>()
</script>

<template>
  <article
    class="relative aspect-[4/3] min-w-0 overflow-hidden rounded-rune-tile bg-rune-surface"
    :class="selected && 'ring-1 ring-white/40'"
  >
    <TaskPreview :run="run" fill @open="$emit('open')" />
    <div class="pointer-events-none absolute top-rune-1 right-rune-1">
      <div class="pointer-events-auto rounded-full bg-black/60">
        <slot name="actions" />
      </div>
    </div>
    <div
      class="pointer-events-none absolute inset-x-0 bottom-0 flex flex-col gap-rune-1 bg-gradient-to-t from-black via-black/80 to-transparent p-rune-2 pt-rune-3"
    >
      <div class="flex items-start gap-2">
        <RuneDot v-if="state" :state="state" class="mt-1.5" />
        <div class="flex min-w-0 flex-col gap-1">
          <span class="line-clamp-2 text-rune-body text-rune-ink">{{ title }}</span>
          <span v-if="meta" class="truncate text-rune-caption text-rune-text-2">{{ meta }}</span>
        </div>
      </div>
      <div class="pointer-events-auto flex items-center gap-rune-1 empty:hidden">
        <slot name="footer" />
      </div>
    </div>
    <div
      v-if="progress !== null"
      role="progressbar"
      :aria-label="`Progress: ${title}`"
      :aria-valuemin="0"
      :aria-valuemax="100"
      :aria-valuenow="progress"
      class="absolute inset-x-0 bottom-0 h-1 bg-white/10"
    >
      <span class="block h-full bg-rune-ink" :style="{ width: `${progress}%` }" />
    </div>
  </article>
</template>