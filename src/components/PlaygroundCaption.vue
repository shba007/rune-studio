<script setup lang="ts">
import { ref, watch } from 'vue'
import { friendly } from '@/config/errors'
import type { CaptionState } from '@/config/playground'
import RuneButton from './RuneButton.vue'

const props = defineProps<{ state: CaptionState }>()
const notice = ref('')
watch(
  () => props.state.result,
  () => (notice.value = ''),
)
async function copy() {
  if (!props.state.result) return
  try {
    await navigator.clipboard.writeText(props.state.result.caption)
    notice.value = 'Copied'
  } catch {
    notice.value = 'Could not copy the caption.'
  }
}
</script>

<template>
  <div
    class="flex min-h-0 w-full flex-1 flex-col items-center gap-4 overflow-y-auto p-5"
    data-testid="caption-result"
  >
    <p v-if="state.status === 'error'" role="alert" class="text-center text-error">
      {{ friendly(state.error) }}
    </p>
    <template v-else-if="state.result">
      <p class="m-0 text-rune-caption text-rune-text-3">
        {{ state.result.model }} · {{ (state.result.elapsedMs / 1000).toFixed(1) }} s
      </p>
      <img
        :src="state.result.image"
        alt="Selected image"
        class="max-h-[50vh] max-w-full rounded-rune-tile object-contain"
      />
      <div class="flex w-full max-w-2xl flex-col gap-3 rounded-rune-tile bg-rune-sunken p-4">
        <p class="m-0 whitespace-pre-wrap break-words text-rune-body text-rune-ink">
          {{ state.result.caption }}
        </p>
        <div class="flex items-center justify-end gap-2">
          <span role="status" class="text-rune-caption text-rune-text-3">{{ notice }}</span
          ><RuneButton size="sm" @click="copy">Copy</RuneButton>
        </div>
      </div>
    </template>
  </div>
</template>