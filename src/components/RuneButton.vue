<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'link'
    size?: 'md' | 'sm'
    type?: 'button' | 'submit'
  }>(),
  { variant: 'secondary', size: 'md', type: 'button' },
)

const VARIANT_CLASS = {
  primary: 'bg-rune-ink text-black hover:bg-white',
  secondary: 'bg-rune-raised text-rune-ink hover:bg-white/12',
  danger: 'bg-attention text-white hover:bg-attention-hover',
  ghost: 'bg-transparent text-rune-text-2 hover:bg-white/8 hover:text-rune-ink',
  link: 'bg-transparent text-rune-text-2 hover:text-rune-ink',
} as const

const SIZE_CLASS = { md: 'h-10 px-5', sm: 'h-8 px-3' } as const
</script>

<template>
  <button
    :type="type"
    class="inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 whitespace-nowrap text-rune-heading uppercase transition-colors disabled:cursor-default disabled:opacity-40"
    :class="[VARIANT_CLASS[variant], variant === 'link' ? '' : [SIZE_CLASS[size], 'rounded-full']]"
  >
    <slot />
  </button>
</template>