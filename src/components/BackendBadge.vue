<script setup lang="ts">
import { computed } from 'vue'

import type { BackendStatus } from '@/types/protocol'

const props = defineProps<{ status: BackendStatus | null }>()

const view = computed(() => {
  const status = props.status
  if (!status) return { tone: 'bg-rune-text-3', text: 'Checking backend…' }
  if (!status.reachable)
    return { tone: 'bg-attention', text: status.error ?? 'Backend unreachable' }
  if (!status.authOk) return { tone: 'bg-attention', text: status.error ?? 'API key rejected' }
  if (status.missingModels.length > 0) {
    return { tone: 'bg-attention', text: `Missing model: ${status.missingModels.join(', ')}` }
  }
  if (status.error) return { tone: 'bg-attention', text: status.error }
  return {
    tone: 'bg-rune-ink',
    text: `Connected${status.latencyMs == null ? '' : ` · ${status.latencyMs} ms`}`,
  }
})
</script>

<template>
  <span
    class="flex min-w-0 items-center gap-2 text-rune-caption text-rune-text-2"
    :title="view.text"
  >
    <span aria-hidden="true" class="size-1.5 shrink-0 rounded-full" :class="view.tone" />
    <span class="break-words">{{ view.text }}</span>
  </span>
</template>