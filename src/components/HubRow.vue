<script setup lang="ts">
defineProps<{ label: string; hint?: string }>()
</script>

<template>
  <div class="flex items-center gap-6 border-b border-white/8 py-4 last:border-b-0">
    <div class="flex min-w-0 flex-1 flex-col gap-1">
      <span class="text-body text-zinc-100">{{ label }}</span>
      <span v-if="hint" class="text-label text-zinc-500">{{ hint }}</span>
    </div>
    <div class="flex min-w-0 shrink-0 items-center gap-2">
      <slot />
    </div>
  </div>
</template>