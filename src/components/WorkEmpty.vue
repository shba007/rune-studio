<script setup lang="ts">
defineProps<{ title: string; hint?: string }>()
</script>

<template>
  <div class="flex flex-col items-start gap-2 rounded-rune-tile bg-rune-surface p-rune-3">
    <p class="m-0 text-rune-body text-rune-ink">{{ title }}</p>
    <p v-if="hint" class="m-0 max-w-lg text-rune-caption text-rune-text-3">{{ hint }}</p>
    <slot />
  </div>
</template>