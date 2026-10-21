<script setup lang="ts">
import AppIcon from './AppIcon.vue'

defineProps<{
  glyph: string | null
  logo?: string | null
  mono?: boolean
  letter?: string | null
  own?: boolean
}>()
</script>

<template>
  <span class="inline-grid size-4 shrink-0 place-items-center" aria-hidden="true">
    <img v-if="logo" :src="logo" alt="" class="size-4" :class="{ invert: mono }" />
    <span v-else-if="letter && !glyph" class="text-caption leading-none font-bold">{{
      letter
    }}</span>
    <svg v-else-if="glyph" viewBox="0 0 24 24" class="size-4" fill="currentColor">
      <path :d="glyph" />
    </svg>
    <AppIcon v-else :name="own ? 'logo' : 'plug'" />
  </span>
</template>