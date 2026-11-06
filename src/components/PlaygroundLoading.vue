<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

import RuneButton from './RuneButton.vue'
import RuneLoader from './RuneLoader.vue'

const props = defineProps<{
  startedAt: number
  label: string
  cancellable: boolean
  compact?: boolean
}>()
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
    class="flex items-center text-center"
    :class="compact ? 'flex-row gap-3' : 'flex-col gap-4'"
    role="status"
    data-testid="playground-loading"
  >
    <RuneLoader />
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