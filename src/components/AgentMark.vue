<script setup lang="ts">
import AppIcon from './AppIcon.vue'

/**
 * The mark of whoever is in control: the brand's single-colour glyph in the current text colour, a
 * neutral plug for an agent with no mark, or Rune's own mark when no outside agent is in control.
 */
defineProps<{ glyph: string | null; own?: boolean }>()
</script>

<template>
  <span class="inline-grid size-4 shrink-0 place-items-center" aria-hidden="true">
    <svg v-if="glyph" viewBox="0 0 24 24" class="size-4" fill="currentColor">
      <path :d="glyph" />
    </svg>
    <AppIcon v-else :name="own ? 'logo' : 'plug'" />
  </span>
</template>