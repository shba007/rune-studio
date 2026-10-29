<script setup lang="ts">
import type { Settings } from '@/types/protocol'
import HubRow from './HubRow.vue'
import RuneChoice from './RuneChoice.vue'
defineProps<{ draft: Settings }>()

const REVIEW: { value: Settings['offerReview']; label: string }[] = [
  { value: 'always', label: 'Always' },
  { value: 'failures', label: 'Failures' },
  { value: 'never', label: 'Never' },
]
const VIDEO: { value: Settings['recordSessionVideo']; label: string }[] = [
  { value: 'off', label: 'Off' },
  { value: 'motion', label: 'Motion' },
]
</script>

<template>
  <HubRow label="Offer task review" hint="Choose when Rune offers a review after a task.">
    <RuneChoice v-model="draft.offerReview" label="Offer task review" :options="REVIEW" />
  </HubRow>
  <HubRow
    label="Task video"
    hint="Motion records the screen at 2 frames per second; its cost is unmeasured."
  >
    <RuneChoice v-model="draft.recordSessionVideo" label="Task video" :options="VIDEO" />
  </HubRow>
</template>