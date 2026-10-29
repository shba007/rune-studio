<script setup lang="ts">
defineProps<{ label: string; hint?: string }>()
</script>

<template>
  <div
    class="flex flex-wrap items-center gap-x-8 gap-y-rune-2 rounded-rune-tile bg-rune-surface p-rune-2 not-first:mt-rune-1"
  >
    <div class="flex min-w-0 flex-1 flex-col gap-1">
      <span class="text-rune-body text-rune-ink">{{ label }}</span>
      <span v-if="hint" class="max-w-lg font-rune-reading text-rune-caption text-rune-text-3">{{
        hint
      }}</span>
    </div>
    <div class="flex min-w-0 max-w-full shrink-0 flex-wrap items-center gap-2">
      <slot />
    </div>
  </div>
</template>