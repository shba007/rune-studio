<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

/**
 * Asked once a session has ended, before the review offer: did it really complete the task? A No asks
 * why (one line, optional). What is said goes into the session's record.
 */
const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const offer = computed(() => agent.reviewOffer.value)
const asking = computed(() => agent.reviewStage.value === 'why')
const reason = ref('')
const field = ref<HTMLTextAreaElement | null>(null)

watch(asking, async (on) => {
  if (!on) return
  reason.value = ''
  await nextTick()
  field.value?.focus()
})

const CHIP = {
  succeeded: 'border-white/30 text-zinc-200',
  stopped: 'border-white/30 text-zinc-300',
  failed: 'border-[#d71921]/60 text-[#ff6b72]',
} as const
</script>

<template>
  <section
    v-if="offer"
    class="flex w-full flex-col gap-2 rounded-[28px] border border-white/15 bg-[#141414] p-4 text-zinc-100"
    role="alertdialog"
    aria-label="Was the task completed?"
  >
    <header class="flex h-6 items-center gap-2">
      <span
        class="rounded-full border px-2 text-caption uppercase"
        :class="CHIP[offer.outcome]"
        data-testid="outcome"
        >{{ offer.outcome }}</span
      >
      <span class="min-w-0 flex-1 truncate text-caption text-zinc-500" :title="offer.task">{{
        offer.task
      }}</span>
    </header>

    <template v-if="!asking">
      <p class="m-0 text-body" data-testid="question">Did this complete your task?</p>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 bg-transparent text-label text-zinc-200 uppercase transition hover:bg-white/10"
          data-testid="no"
          @click="agent.answerComplete(false)"
        >
          No
        </button>
        <button
          type="button"
          class="h-8 flex-1 cursor-pointer rounded-full border-0 bg-[#f1f1ef] text-label text-[#111] uppercase transition hover:bg-white"
          data-testid="yes"
          @click="agent.answerComplete(true)"
        >
          Yes
        </button>
      </div>
      <button
        type="button"
        class="h-6 cursor-pointer self-center border-0 bg-transparent text-caption text-zinc-500 uppercase transition hover:text-zinc-300"
        data-testid="skip"
        @click="agent.answerComplete(null)"
      >
        Skip
      </button>
    </template>

    <template v-else>
      <p class="m-0 text-body">What went wrong?</p>
      <textarea
        ref="field"
        v-model="reason"
        rows="2"
        placeholder="A few words help"
        class="min-h-14 w-full resize-none rounded-2xl border-0 bg-white/8 px-3 py-2 text-body text-zinc-100 outline-none placeholder:text-zinc-500 focus-visible:ring-2 focus-visible:ring-white/40"
        data-testid="why"
        @keydown.enter.exact.prevent="agent.answerWhy(reason)"
      />
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 bg-transparent text-label text-zinc-200 uppercase transition hover:bg-white/10"
          data-testid="skip-why"
          @click="agent.answerWhy('')"
        >
          Skip
        </button>
        <button
          type="button"
          class="h-8 flex-1 cursor-pointer rounded-full border-0 bg-[#f1f1ef] text-label text-[#111] uppercase transition hover:bg-white"
          data-testid="send-why"
          @click="agent.answerWhy(reason)"
        >
          Send
        </button>
      </div>
    </template>
  </section>
</template>