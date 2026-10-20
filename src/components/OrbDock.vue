<script setup lang="ts">
import type { Agent } from '@/composables/useAgent'

import AppIcon from './AppIcon.vue'

defineProps<{ agent: Agent }>()
</script>

<template>
  <!-- A capsule as tall as the orb (64 px) with its two tools stacked: settings above, screen analysis below. -->
  <div
    data-tauri-drag-region
    class="flex h-16 w-8 shrink-0 flex-col items-center justify-center rounded-full bg-[#141414] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.08)]"
  >
    <button
      type="button"
      class="grid size-8 cursor-pointer place-items-center rounded-full text-zinc-300 transition hover:bg-white/10 hover:text-white"
      aria-label="Settings"
      title="Settings"
      @click="agent.setMode('settings')"
    >
      <AppIcon name="tune" />
    </button>
    <button
      type="button"
      class="grid size-8 cursor-pointer place-items-center rounded-full text-zinc-300 transition hover:bg-white/10 hover:text-white disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent"
      aria-label="Analyse the screen"
      title="Analyse the screen"
      :disabled="agent.isRunning.value"
      @click="agent.analyzeScreen()"
    >
      <AppIcon name="scan" />
    </button>
  </div>
</template>