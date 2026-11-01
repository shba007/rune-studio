<script setup lang="ts">
import { friendly } from '@/config/errors'
import type { SpeechState } from '@/config/playground'

defineProps<{ state: SpeechState }>()
</script>

<template>
  <div class="w-full max-w-2xl p-6" data-testid="speech-result">
    <p
      v-if="state.status === 'running'"
      role="status"
      class="text-center text-rune-body text-rune-text-2"
    >
      Creating voice…
    </p>
    <p
      v-else-if="state.status === 'error'"
      role="alert"
      class="text-center text-rune-body text-error"
    >
      {{ friendly(state.error) }}
    </p>
    <div v-else-if="state.result" class="flex flex-col gap-3 rounded-rune-tile bg-rune-sunken p-5">
      <p class="m-0 whitespace-pre-wrap break-words text-rune-body text-rune-ink">
        {{ state.result.text }}
      </p>
      <p class="m-0 text-rune-caption text-rune-text-3">
        {{ state.result.model }} · {{ (state.result.elapsedMs / 1000).toFixed(1) }} s
      </p>
      <audio
        :src="state.result.audio"
        controls
        preload="metadata"
        class="w-full"
        data-testid="speech-player"
      >
        Your browser does not support audio playback.
      </audio>
      <a
        :href="state.result.audio"
        download="rune-voice.wav"
        class="self-end text-rune-body text-rune-ink underline"
        data-testid="speech-download"
        >Download WAV</a
      >
    </div>
  </div>
</template>