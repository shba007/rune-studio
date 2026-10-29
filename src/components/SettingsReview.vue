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
      class="rounded-rune-tile bg-rune-sunken px-rune-2 py-2 text-rune-body text-rune-ink"
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
      class="rounded-rune-tile bg-rune-sunken px-rune-2 py-2 text-rune-body text-rune-ink"
    >
      <option value="off">Off</option>
      <option value="motion">Motion</option>
    </select>
  </HubRow>
</template>