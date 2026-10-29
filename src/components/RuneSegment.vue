<script setup lang="ts">
withDefaults(defineProps<{ selected?: boolean; type?: 'button' | 'submit' }>(), {
  selected: false,
  type: 'button',
})
</script>

<template>
  <button
    :type="type"
    class="inline-flex h-6 shrink-0 cursor-pointer items-center justify-center gap-1 rounded-full px-3 text-rune-heading uppercase transition-colors disabled:cursor-default disabled:opacity-40"
    :class="
      selected
        ? 'bg-rune-ink text-black'
        : 'bg-transparent text-rune-text-2 hover:bg-white/8 hover:text-rune-ink'
    "
  >
    <slot />
  </button>
</template>