<script setup lang="ts">
import { friendly } from '@/config/errors'
import type { DepthState } from '@/config/playground'

import PlaygroundCompare from './PlaygroundCompare.vue'
import RuneChip from './RuneChip.vue'

defineProps<{ state: DepthState }>()

const seconds = (ms: number) => `${(ms / 1000).toFixed(1)} s`
</script>

<template>
  <div
    class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-y-auto p-4"
    data-testid="depth-result"
  >
    <header class="flex min-h-8 items-center justify-center gap-2">
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

    <p
      v-if="state.status === 'error'"
      role="alert"
      class="m-0 max-w-prose self-center text-center text-rune-body text-error"
    >
      {{ friendly(state.error) }}
    </p>

    <figure v-if="state.result" class="m-0 flex min-w-0 flex-col items-center gap-2">
      <PlaygroundCompare
        :original="state.result.original"
        :output="state.result.preview"
        label="Relative depth map"
        :width="state.result.width"
        :height="state.result.height"
      />
      <p v-if="!state.result.preview" class="m-0 text-rune-body text-rune-text-2">
        Infer returned no preview image.
      </p>
      <figcaption class="text-rune-caption text-rune-text-3">
        Depth map left, source right. Relative depth, not meters. Range
        {{ state.result.min.toFixed(2) }} to {{ state.result.max.toFixed(2) }}.
      </figcaption>
    </figure>
  </div>
</template>