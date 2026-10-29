<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    variant?: 'neutral' | 'primary' | 'voice' | 'danger' | 'ghost'
    size?: 'md' | 'sm' | 'xs'
    type?: 'button' | 'submit'
  }>(),
  { variant: 'neutral', size: 'md', type: 'button' },
)

const VARIANT_CLASS = {
  neutral: 'bg-rune-raised text-rune-ink hover:bg-white/12',
  primary: 'bg-rune-ink text-black hover:bg-white',
  voice: 'bg-voice text-white hover:bg-voice-hover',
  danger: 'bg-attention text-white hover:bg-attention-hover',
  ghost: 'bg-transparent text-rune-text-2 hover:bg-white/8 hover:text-rune-ink',
} as const

const SIZE_CLASS = { md: 'size-10', sm: 'size-8', xs: 'size-6' } as const
</script>

<template>
  <button
    :type="type"
    :aria-label="label"
    :title="label"
    class="inline-grid shrink-0 cursor-pointer place-items-center rounded-full p-0 transition-colors disabled:cursor-default disabled:opacity-40"
    :class="[VARIANT_CLASS[variant], SIZE_CLASS[size]]"
  >
    <slot />
  </button>
</template>