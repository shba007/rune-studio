<script setup lang="ts">
defineProps<{ value: number | null; label: string; detail?: string }>()
</script>
<template>
  <div class="min-w-0">
    <div class="mb-2 flex items-center justify-between gap-3 text-rune-caption">
      <span class="text-rune-text-3">{{ label }}</span
      ><span class="text-rune-text-2 tabular-nums">{{
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
      class="h-1.5 overflow-hidden rounded-full bg-rune-sunken"
    >
      <span
        v-if="value !== null"
        class="block h-full rounded-full bg-rune-ink"
        :style="{ width: `${Math.max(0, Math.min(100, value))}%` }"
      />
    </div>
    <p v-if="detail" class="mt-2 mb-0 text-rune-caption text-rune-text-3">
      {{ detail }}
    </p>
  </div>
</template>