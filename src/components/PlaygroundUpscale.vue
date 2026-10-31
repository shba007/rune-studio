<script setup lang="ts">
import { friendly } from '@/config/errors'
import type { UpscaleState } from '@/config/playground'

import PlaygroundCompare from './PlaygroundCompare.vue'
import RuneChip from './RuneChip.vue'

defineProps<{ state: UpscaleState }>()

const seconds = (ms: number) => `${(ms / 1000).toFixed(1)} s`
</script>

<template>
  <div
    class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-y-auto p-4"
    data-testid="upscale-result"
  >
    <header class="flex min-h-8 items-center justify-center gap-2">
      <RuneChip :variant="state.status === 'error' ? 'red' : 'neutral'" data-testid="upscale-stage">
        {{
          state.status === 'running' ? 'Upscaling' : state.status === 'error' ? 'Failed' : 'Done'
        }}
      </RuneChip>
      <span
        v-if="state.result"
        class="min-w-0 truncate text-rune-caption text-rune-text-2"
        data-testid="upscale-meta"
      >
        {{ state.result.model }} · {{ state.result.scale }}× · {{ seconds(state.result.elapsedMs) }}
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
        :output="state.result.output"
        label="Upscaled image"
        :width="state.result.outputWidth"
        :height="state.result.outputHeight"
      />
      <figcaption class="text-rune-caption text-rune-text-3">
        Upscaled left, source right.
        {{ state.result.inputWidth }}×{{ state.result.inputHeight }} to
        {{ state.result.outputWidth }}×{{ state.result.outputHeight }}.
      </figcaption>
    </figure>
  </div>
</template>