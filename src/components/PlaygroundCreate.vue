<script setup lang="ts">
import { friendly } from '@/config/errors'
import type { CreateState } from '@/config/playground'

import RuneChip from './RuneChip.vue'

defineProps<{ state: CreateState }>()

const seconds = (ms: number) => `${(ms / 1000).toFixed(1)} s`
</script>

<template>
  <div
    class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-y-auto p-4"
    data-testid="create-result"
  >
    <header class="flex min-h-8 items-center justify-center gap-2">
      <RuneChip :variant="state.status === 'error' ? 'red' : 'neutral'" data-testid="create-stage">
        {{ state.status === 'running' ? 'Creating' : state.status === 'error' ? 'Failed' : 'Done' }}
      </RuneChip>
      <span
        v-if="state.result"
        class="min-w-0 truncate text-rune-caption text-rune-text-2"
        data-testid="create-meta"
      >
        {{ state.result.model }} · {{ state.result.size }} · {{ seconds(state.result.elapsedMs) }}
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
      <img
        :src="state.result.image"
        :alt="state.result.prompt"
        class="block max-h-[60vh] max-w-full rounded-rune-tile object-contain"
        data-testid="create-image"
      />
      <figcaption class="max-w-prose text-center text-rune-caption text-rune-text-3">
        {{ state.result.prompt }}
      </figcaption>
    </figure>
  </div>
</template>