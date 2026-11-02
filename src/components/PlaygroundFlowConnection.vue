<script setup lang="ts">
defineProps<{ label: string; tone: 'sky' | 'violet' }>()
</script>
<template>
  <div class="relative flex h-16 w-full shrink-0 items-center justify-center" aria-hidden="true">
    <span class="h-full border-l border-dashed border-rune-text-3/50"></span>
    <span
      class="absolute left-[calc(50%+12px)] rounded-rune-tile bg-rune-surface px-2 py-0.5 text-[10px] uppercase text-rune-text-3"
      >{{ label }}</span
    >
  </div>
</template>