<script setup lang="ts">
import { computed } from 'vue'

import type { BackendStatus } from '@/types/protocol'

const props = defineProps<{ status: BackendStatus | null }>()

const view = computed(() => {
  const status = props.status
  if (!status) return { tone: 'bg-zinc-500', text: 'Checking backend…' }
  if (!status.reachable) return { tone: 'bg-red-500', text: status.error ?? 'Backend unreachable' }
  if (!status.authOk) return { tone: 'bg-amber-400', text: status.error ?? 'API key rejected' }
  if (status.missingModels.length > 0) {
    return { tone: 'bg-amber-400', text: `Missing model: ${status.missingModels.join(', ')}` }
  }
  if (status.error) return { tone: 'bg-amber-400', text: status.error }
  return { tone: 'bg-emerald-500', text: `Backend OK · ${status.latencyMs ?? '?'} ms` }
})
</script>

<template>
  <span class="flex min-w-0 items-center gap-1.5 text-label text-zinc-400" :title="view.text">
    <span class="size-2 shrink-0 rounded-full" :class="view.tone" />
    <span class="truncate">{{ view.text }}</span>
  </span>
</template>