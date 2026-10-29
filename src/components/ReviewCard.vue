<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { useIntervalFn, useNow } from '@vueuse/core'
import { computed, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { REVIEW_SECONDS, dotsLeft, reviewMeta } from '@/config/review'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const now = useNow({ scheduler: (update) => useIntervalFn(update, 250) })
const offer = computed(() => agent.reviewOffer.value)
const lit = computed(() => (offer.value ? dotsLeft(now.value.getTime(), offer.value.endsAt) : 0))
watch(
  lit,
  (left) => {
    if (offer.value && left === 0) agent.dismissReview()
  },
  { immediate: true },
)

const CHIP = {
  succeeded: 'text-rune-ink',
  stopped: 'text-rune-text-2',
  failed: 'bg-attention/20 text-error',
} as const
</script>

<template>
  <section
    v-if="offer"
    class="flex w-full flex-col gap-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-ink"
    role="alertdialog"
    aria-label="Review the task"
  >
    <header class="flex h-6 items-center gap-2">
      <span
        class="rounded-full px-2 text-rune-caption uppercase"
        :class="CHIP[offer.outcome]"
        data-testid="outcome"
        >{{ offer.outcome }}</span
      >
      <span class="ml-auto text-rune-caption text-rune-text-2" data-testid="meta">{{
        reviewMeta(offer.durationSecs, offer.steps)
      }}</span>
    </header>
    <p class="m-0 truncate text-rune-body" :title="offer.task">{{ offer.task }}</p>
    <div class="flex items-center gap-2">
      <RuneButton size="sm" class="flex-1" @click="agent.dismissReview()"> Dismiss </RuneButton>
      <RuneButton variant="primary" size="sm" class="flex-1" @click="agent.openReview(offer.runId)">
        Review
      </RuneButton>
    </div>
    <div class="flex gap-1" aria-hidden="true" data-testid="dots">
      <span
        v-for="n in REVIEW_SECONDS"
        :key="n"
        class="size-1 rounded-full bg-primary motion-safe:transition-opacity"
        :class="n <= lit ? 'opacity-80' : 'opacity-15'"
      />
    </div>
  </section>
</template>