<script setup lang="ts">
withDefaults(defineProps<{ state?: 'idle' | 'ok' | 'warn' | 'down' | 'busy' }>(), {
  state: 'idle',
})

const STATE_CLASS = {
  idle: 'bg-rune-text-3',
  ok: 'bg-rune-ink',
  warn: 'bg-rune-ink/45',
  down: 'bg-attention',
  busy: 'bg-rune-ink motion-safe:animate-pulse',
} as const
</script>

<template>
  <span class="inline-block size-2 shrink-0 rounded-full" :class="STATE_CLASS[state]" />
</template>