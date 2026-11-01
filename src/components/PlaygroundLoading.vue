<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import RuneButton from './RuneButton.vue'

const props = defineProps<{ startedAt: number; label: string; cancellable: boolean }>()
defineEmits<{ cancel: [] }>()

const now = ref(Date.now())
watch(
  () => props.startedAt,
  () => (now.value = Date.now()),
)
useIntervalFn(() => (now.value = Date.now()), 100)
const elapsed = computed(() => (Math.max(0, now.value - props.startedAt) / 1000).toFixed(1))
</script>

<template>
  <div
    class="flex flex-col items-center gap-4 text-center"
    role="status"
    data-testid="playground-loading"
  >
    <div class="loader-grid" aria-hidden="true"><span v-for="n in 4" :key="n" /></div>
    <p class="m-0 text-rune-body text-rune-text-2">
      {{ label }}… <span data-testid="loading-elapsed">{{ elapsed }} s</span>
    </p>
    <RuneButton
      v-if="cancellable"
      size="sm"
      variant="danger"
      data-testid="result-cancel"
      @click="$emit('cancel')"
      >Cancel</RuneButton
    >
  </div>
</template>

<style scoped>
.loader-grid {
  display: grid;
  grid-template-columns: repeat(2, 8px);
  gap: 5px;
}
.loader-grid span {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  background: currentColor;
  color: var(--color-rune-text-2);
  animation: pulse 1.2s ease-in-out infinite;
}
.loader-grid span:nth-child(2) {
  animation-delay: 0.15s;
}
.loader-grid span:nth-child(4) {
  animation-delay: 0.3s;
}
.loader-grid span:nth-child(3) {
  animation-delay: 0.45s;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 0.25;
  }
  50% {
    opacity: 1;
  }
}
@media (prefers-reduced-motion: reduce) {
  .loader-grid span {
    animation: none;
    opacity: 0.7;
  }
}
</style>