<script setup lang="ts">
import { personalitySymbol } from '@/config/personalitySymbols'
defineProps<{ mark: string }>()
</script>
<template>
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="personalitySymbol(mark).path" />
  </svg>
</template>