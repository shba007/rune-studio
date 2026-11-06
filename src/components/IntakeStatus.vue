<script setup lang="ts">
import RuneLoader from './RuneLoader.vue'

defineProps<{ classifying: boolean; error: string }>()
</script>

<template>
  <div
    v-if="classifying"
    role="status"
    aria-live="polite"
    class="flex items-center gap-3 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-body text-rune-text-2"
  >
    <RuneLoader />
    <span>Understanding your request…</span>
  </div>
  <p
    v-if="error"
    role="alert"
    class="m-0 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-caption text-error"
  >
    {{ error }}
  </p>
</template>