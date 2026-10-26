<script setup lang="ts">
defineProps<{ value: number | null; label: string; detail?: string }>()
</script>
<template>
  <div class="min-w-0">
    <div class="mb-2 flex items-center justify-between gap-3 text-caption">
      <span class="text-zinc-400">{{ label }}</span
      ><span class="text-zinc-300 tabular-nums">{{
        value === null ? 'Not measured' : `${value}%`
      }}</span>
    </div>
    <div
      role="progressbar"
      :aria-label="label"
      :aria-valuemin="0"
      :aria-valuemax="100"
      :aria-valuenow="value ?? undefined"
      :aria-valuetext="value === null ? 'Not measured' : `${value}%`"
      class="h-1.5 overflow-hidden rounded-full bg-white/8"
    >
      <span
        v-if="value !== null"
        class="block h-full rounded-full bg-zinc-300"
        :style="{ width: `${Math.max(0, Math.min(100, value))}%` }"
      />
    </div>
    <p v-if="detail" class="mt-2 mb-0 text-caption leading-relaxed text-zinc-500">{{ detail }}</p>
  </div>
</template>