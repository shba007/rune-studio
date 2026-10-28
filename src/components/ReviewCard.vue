<script setup lang="ts">
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
  succeeded: 'border-white/30 text-zinc-200',
  stopped: 'border-white/30 text-zinc-300',
  failed: 'border-attention/60 text-error',
} as const
</script>

<template>
  <section
    v-if="offer"
    class="flex w-full flex-col gap-2 rounded-rune-panel border border-rune-outline bg-rune-surface p-rune-2 font-rune-reading text-zinc-100"
    role="alertdialog"
    aria-label="Review the task"
  >
    <header class="flex h-6 items-center gap-2">
      <span
        class="rounded-full border px-2 text-rune-caption uppercase"
        :class="CHIP[offer.outcome]"
        data-testid="outcome"
        >{{ offer.outcome }}</span
      >
      <span class="ml-auto text-rune-caption text-zinc-500" data-testid="meta">{{
        reviewMeta(offer.durationSecs, offer.steps)
      }}</span>
    </header>
    <p class="m-0 truncate text-rune-body" :title="offer.task">{{ offer.task }}</p>
    <div class="flex items-center gap-2">
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 text-rune-caption text-zinc-200 uppercase transition hover:bg-white/10"
        @click="agent.dismissReview()"
      >
        Dismiss
      </button>
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full bg-primary text-rune-caption text-[#111] uppercase transition hover:bg-white"
        @click="agent.openReview(offer.runId)"
      >
        Review
      </button>
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