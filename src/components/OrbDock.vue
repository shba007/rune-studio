<script setup lang="ts">
import type { Agent } from '@/composables/useAgent'

import AppIcon from './AppIcon.vue'

defineProps<{ agent: Agent }>()

const tool =
  'absolute left-1 grid size-7 cursor-pointer place-items-center rounded-full bg-[#141414] text-zinc-300 shadow-[inset_0_0_0_1px_rgb(255_255_255/0.08)] transition hover:bg-[#222] hover:text-(--accent) disabled:cursor-default disabled:opacity-40 disabled:hover:bg-[#141414]'
</script>

<template>
  <div class="relative h-16 w-8 shrink-0">
    <button
      type="button"
      :class="[tool, 'top-0']"
      aria-label="Settings"
      title="Settings"
      @click="agent.openSettings()"
    >
      <AppIcon name="tune" />
    </button>
    <button
      type="button"
      :class="[tool, 'bottom-0']"
      aria-label="Analyse the screen"
      :title="
        agent.settings.value?.externalAgent
          ? 'Ask your external agent to call rune_analyze'
          : 'Analyse the screen'
      "
      :disabled="agent.isRunning.value || agent.settings.value?.externalAgent"
      @click="agent.analyzeScreen()"
    >
      <AppIcon name="scan" />
    </button>
  </div>
</template>