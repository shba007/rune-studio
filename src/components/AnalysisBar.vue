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
    <AgentMark :glyph="null" own class="text-[#f1f1ef]" />
    <span class="text-caption whitespace-nowrap uppercase">Screen analysis</span>
    <div class="flex items-center gap-2 border-l border-white/20 pl-3">
      <span
        v-if="stage"
        class="text-caption whitespace-nowrap uppercase"
        :class="failed ? 'text-[#ff6b72]' : 'text-[#f1f1ef]'"
        data-testid="stage"
        >{{ stage }}</span
      >
      <span class="text-label whitespace-nowrap text-white/70 tabular-nums" data-testid="timer">{{
        formatElapsed(elapsedMs)
      }}</span>
    </div>
    <div class="flex items-center gap-2 border-l border-white/20 pr-3 pl-3">
      <span v-if="hotkey" class="text-caption whitespace-nowrap text-white/50 uppercase">
        {{ hotkey }} cancels
      </span>
    </div>
  </OverlayBarShell>
</template>