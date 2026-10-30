<script setup lang="ts">
import AppIcon from './AppIcon.vue'
import RuneIconButton from './RuneIconButton.vue'

defineProps<{ label: string }>()
defineEmits<{ close: [] }>()
</script>

<template>
  <aside
    :aria-label="label"
    class="glass @container flex w-[28rem] max-w-[55%] shrink-0 flex-col overflow-y-auto bg-rune-sunken"
    data-testid="peek"
  >
    <header
      class="sticky top-0 z-10 flex items-center justify-between gap-rune-1 bg-rune-sunken p-rune-1"
    >
      <RuneIconButton variant="ghost" size="sm" label="Close panel" @click="$emit('close')">
        <AppIcon name="close" size="sm" />
      </RuneIconButton>
      <slot name="header" />
    </header>
    <div class="flex flex-col gap-rune-3 px-rune-3 pt-rune-1 pb-rune-3">
      <slot />
    </div>
  </aside>
</template>