<script setup lang="ts">
import { friendly } from '@/config/errors'
import type { EditState } from '@/config/playground'

defineProps<{ state: EditState; variation?: boolean; reference?: boolean }>()
</script>

<template>
  <div
    class="flex min-h-0 w-full flex-1 flex-col items-center gap-4 overflow-y-auto p-4"
    data-testid="edit-result"
  >
    <p v-if="state.status === 'error'" role="alert" class="text-center text-error">
      {{ friendly(state.error) }}
    </p>
    <template v-if="state.result">
      <p class="m-0 text-rune-caption text-rune-text-2">
        {{ state.result.model }} · {{ (state.result.elapsedMs / 1000).toFixed(1) }} s
      </p>
      <img
        :src="state.result.image"
        :alt="variation ? 'Image variation' : state.result.prompt"
        class="max-h-[60vh] max-w-full rounded-rune-tile object-contain"
        data-testid="edit-image"
      />
      <p
        v-if="state.result.prompt && !reference"
        class="m-0 text-center text-rune-caption text-rune-text-3"
      >
        {{ state.result.prompt }}
      </p>
      <details
        v-if="state.result.prompt && reference"
        class="max-w-prose text-center text-rune-caption text-rune-text-3"
      >
        <summary class="cursor-pointer">View prompt</summary>
        <p class="m-0 whitespace-pre-wrap break-words pt-2">{{ state.result.prompt }}</p>
      </details>
      <a
        :href="state.result.image"
        :download="`rune-${reference ? 'reference' : variation ? 'variation' : 'edit'}.${state.result.image.startsWith('data:image/jpeg') ? 'jpg' : 'png'}`"
        class="text-rune-body text-rune-ink underline"
        data-testid="edit-download"
        >Download image</a
      >
    </template>
  </div>
</template>