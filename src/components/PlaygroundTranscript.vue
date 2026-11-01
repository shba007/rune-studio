<script setup lang="ts">
import { ref, watch } from 'vue'
import { friendly } from '@/config/errors'
import type { TranscribeState } from '@/config/playground'
import RuneButton from './RuneButton.vue'

const props = defineProps<{ state: TranscribeState }>()
const notice = ref('')
watch(
  () => props.state.result,
  () => (notice.value = ''),
)
async function copy() {
  if (!props.state.result) return
  try {
    await navigator.clipboard.writeText(props.state.result.text)
    notice.value = 'Copied'
  } catch {
    notice.value = 'Could not copy the transcript.'
  }
}
</script>
<template>
  <div class="w-full max-w-2xl p-6" data-testid="transcript-result">
    <p v-if="state.status === 'error'" role="alert" class="text-center text-error">
      {{ friendly(state.error) }}
    </p>
    <div v-else-if="state.result" class="flex flex-col gap-3 rounded-rune-tile bg-rune-sunken p-5">
      <p class="m-0 text-rune-caption text-rune-text-3">
        {{ state.result.model }} · {{ (state.result.elapsedMs / 1000).toFixed(1) }} s
      </p>
      <p
        class="m-0 max-h-[50vh] overflow-y-auto whitespace-pre-wrap break-words text-rune-body text-rune-ink"
      >
        {{ state.result.text || 'No speech was detected.' }}
      </p>
      <div class="flex items-center justify-end gap-2">
        <span role="status" class="text-rune-caption text-rune-text-3">{{ notice }}</span
        ><RuneButton size="sm" :disabled="!state.result.text" @click="copy">Copy</RuneButton>
      </div>
    </div>
  </div>
</template>