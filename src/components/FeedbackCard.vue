<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed, nextTick, ref, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

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
  succeeded: 'text-rune-ink',
  stopped: 'text-rune-text-2',
  failed: 'text-error',
} as const
</script>

<template>
  <section
    v-if="offer"
    class="flex w-full flex-col gap-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-ink"
    role="alertdialog"
    aria-label="Was the task completed?"
  >
    <header class="flex h-6 items-center gap-2">
      <span
        class="rounded-full px-2 text-rune-caption uppercase"
        :class="CHIP[offer.outcome]"
        data-testid="outcome"
        >{{ offer.outcome }}</span
      >
      <span
        class="min-w-0 flex-1 truncate text-rune-caption text-rune-text-2"
        :title="offer.task"
        >{{ offer.task }}</span
      >
    </header>

    <template v-if="!asking">
      <p class="m-0 text-rune-body" data-testid="question">Did this complete your task?</p>
      <div class="flex items-center gap-2">
        <RuneButton
          variant="ghost"
          size="sm"
          class="flex-1"
          data-testid="no"
          @click="agent.answerComplete(false)"
        >
          No
        </RuneButton>
        <RuneButton
          variant="primary"
          size="sm"
          class="flex-1"
          data-testid="yes"
          @click="agent.answerComplete(true)"
        >
          Yes
        </RuneButton>
      </div>
      <RuneButton
        variant="link"
        class="self-center"
        data-testid="skip"
        @click="agent.answerComplete(null)"
      >
        Skip
      </RuneButton>
    </template>

    <template v-else>
      <p class="m-0 text-rune-body">What went wrong?</p>
      <textarea
        ref="field"
        v-model="reason"
        rows="2"
        placeholder="A few words help"
        class="min-h-14 w-full resize-none rounded-rune-tile bg-rune-surface px-3 py-2 text-rune-body text-rune-ink placeholder:text-rune-text-3"
        data-testid="why"
        @keydown.enter.exact.prevent="agent.answerWhy(reason)"
      />
      <div class="flex items-center gap-2">
        <RuneButton
          variant="ghost"
          size="sm"
          class="flex-1"
          data-testid="skip-why"
          @click="agent.answerWhy('')"
        >
          Skip
        </RuneButton>
        <RuneButton
          variant="primary"
          size="sm"
          class="flex-1"
          data-testid="send-why"
          @click="agent.answerWhy(reason)"
        >
          Send
        </RuneButton>
      </div>
    </template>
  </section>
</template>