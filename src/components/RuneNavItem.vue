<script setup lang="ts">
withDefaults(defineProps<{ selected?: boolean }>(), { selected: false })
</script>

<template>
  <button
    type="button"
    class="flex h-10 w-full cursor-pointer items-center gap-rune-2 rounded-rune-tile px-rune-2 text-left text-rune-body transition-colors disabled:cursor-default disabled:opacity-40"
    :class="
      selected
        ? 'bg-rune-raised text-rune-ink'
        : 'bg-transparent text-rune-text-2 hover:bg-white/5 hover:text-rune-ink'
    "
    :aria-current="selected ? 'page' : undefined"
  >
    <slot />
  </button>
</template>