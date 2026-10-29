<script setup lang="ts">
import { computed } from 'vue'

import { formatElapsed, stageWord } from '@/config/analysis'
import type { AnalysisStatus } from '@/types/protocol'

import AgentMark from './AgentMark.vue'
import OverlayBarShell from './OverlayBarShell.vue'

const props = defineProps<{ status: AnalysisStatus; elapsedMs: number; hotkey: string }>()

const stage = computed(() => stageWord(props.status))
const failed = computed(() => props.status === 'error')
</script>

<template>
  <OverlayBarShell aria-live="polite">
    <AgentMark :glyph="null" own class="text-rune-ink" />
    <span class="text-rune-heading whitespace-nowrap uppercase">Visual analysis</span>
    <div class="flex items-center gap-2 pl-3">
      <span
        v-if="stage"
        class="text-rune-heading whitespace-nowrap uppercase"
        :class="failed ? 'text-error' : 'text-rune-ink'"
        data-testid="stage"
        >{{ stage }}</span
      >
      <span
        class="text-rune-caption whitespace-nowrap text-rune-text-2 tabular-nums"
        data-testid="timer"
        >{{ formatElapsed(elapsedMs) }}</span
      >
    </div>
    <div class="flex items-center gap-2 pr-3 pl-3">
      <span v-if="hotkey" class="text-rune-heading whitespace-nowrap text-rune-text-3 uppercase">
        {{ hotkey }}
      </span>
    </div>
  </OverlayBarShell>
</template>