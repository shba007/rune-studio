<script setup lang="ts">
import type { Settings } from '@/types/protocol'
import HubRow from './HubRow.vue'
defineProps<{ draft: Settings }>()
</script>

<template>
  <HubRow label="Offer task review" hint="Choose when Rune offers a review after a task.">
    <select
      v-model="draft.offerReview"
      aria-label="Offer task review"
      class="rounded-lg bg-zinc-900 px-3 py-2 text-body text-zinc-100"
    >
      <option value="always">Always</option>
      <option value="failures">Failures</option>
      <option value="never">Never</option>
    </select>
  </HubRow>
  <HubRow
    label="Task video"
    hint="Motion records the screen at 2 frames per second; its cost is unmeasured."
  >
    <select
      v-model="draft.recordSessionVideo"
      aria-label="Task video"
      class="rounded-lg bg-zinc-900 px-3 py-2 text-body text-zinc-100"
    >
      <option value="off">Off</option>
      <option value="motion">Motion</option>
    </select>
  </HubRow>
</template>