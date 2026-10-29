<script setup lang="ts">
import RuneDot from './RuneDot.vue'

withDefaults(defineProps<{ tone?: 'info' | 'error' }>(), { tone: 'info' })
</script>

<template>
  <p
    :role="tone === 'error' ? 'alert' : 'status'"
    class="m-0 flex items-start gap-2 rounded-rune-tile bg-rune-surface px-rune-2 py-rune-1 text-rune-caption"
    :class="tone === 'error' ? 'text-error' : 'text-rune-text-2'"
  >
    <RuneDot :state="tone === 'error' ? 'down' : 'idle'" class="mt-1" />
    <slot />
  </p>
</template>