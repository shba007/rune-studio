<script setup lang="ts">
withDefaults(defineProps<{ variant?: 'neutral' | 'ink' | 'red' }>(), { variant: 'neutral' })

const VARIANT_CLASS = {
  neutral: 'bg-rune-raised text-rune-text-2',
  ink: 'bg-rune-ink text-black',
  red: 'bg-attention text-white',
} as const
</script>

<template>
  <span
    class="inline-flex h-6 shrink-0 items-center gap-1.5 rounded-full px-3 text-rune-heading uppercase"
    :class="VARIANT_CLASS[variant]"
  >
    <slot />
  </span>
</template>