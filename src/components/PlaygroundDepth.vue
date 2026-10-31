<script setup lang="ts">
import { friendly } from '@/config/errors'
import type { DepthState } from '@/config/playground'

import RuneChip from './RuneChip.vue'

defineProps<{ state: DepthState }>()

const seconds = (ms: number) => `${(ms / 1000).toFixed(1)} s`
</script>

<template>
  <div
    class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-y-auto p-4"
    data-testid="depth-result"
  >
    <header class="flex min-h-8 items-center gap-2">
      <RuneChip :variant="state.status === 'error' ? 'red' : 'neutral'" data-testid="depth-stage">
        {{
          state.status === 'running' ? 'Estimating' : state.status === 'error' ? 'Failed' : 'Done'
        }}
      </RuneChip>
      <span
        v-if="state.result"
        class="min-w-0 truncate text-rune-caption text-rune-text-2"
        data-testid="depth-meta"
      >
        {{ state.result.model }} · {{ state.result.width }}×{{ state.result.height }} ·
        {{ seconds(state.result.elapsedMs) }}
      </span>
    </header>

    <p v-if="state.status === 'error'" role="alert" class="m-0 text-rune-body text-error">
      {{ friendly(state.error) }}
    </p>

    <div v-if="state.result" class="grid min-h-0 grid-cols-2 gap-4">
      <figure class="m-0 flex min-w-0 flex-col gap-2">
        <img
          :src="state.result.original"
          alt="Source frame"
          class="block max-h-[56vh] w-full rounded-rune-tile object-contain"
          data-testid="depth-original"
        />
        <figcaption class="text-rune-caption text-rune-text-3">Source</figcaption>
      </figure>
      <figure class="m-0 flex min-w-0 flex-col gap-2">
        <img
          v-if="state.result.preview"
          :src="state.result.preview"
          alt="Relative depth map"
          class="block max-h-[56vh] w-full rounded-rune-tile object-contain"
          data-testid="depth-preview"
        />
        <p v-else class="m-0 text-rune-body text-rune-text-2">Infer returned no preview image.</p>
        <figcaption class="text-rune-caption text-rune-text-3">
          Relative depth, not meters. Range {{ state.result.min.toFixed(2) }} to
          {{ state.result.max.toFixed(2) }}.
        </figcaption>
      </figure>
    </div>
  </div>
</template>