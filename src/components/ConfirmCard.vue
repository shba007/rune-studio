<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

/**
 * The question an action is waiting on: what it wants to do, why it counts as risky, and the two
 * answers. Nothing risky happens without a Yes, and an unanswered question counts as No.
 */
const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const question = computed(() => agent.question.value)
const asking = computed(() => question.value?.kind === 'question')
const left = ref(0)
// Each new question starts its own countdown.
watch(question, (asked) => (left.value = asked?.timeoutSecs ?? 0), { immediate: true })
useIntervalFn(() => (left.value = Math.max(0, left.value - 1)), 1000)

const clock = computed(
  () => `${Math.floor(left.value / 60)}:${String(left.value % 60).padStart(2, '0')}`,
)

// No is focused first, so a stray Enter declines rather than allows.
const no = useTemplateRef<HTMLButtonElement>('no')
onMounted(() => no.value?.focus())
</script>

<template>
  <section
    v-if="question"
    class="flex w-full flex-col gap-2 rounded-[28px] border border-[#d71921]/50 bg-[#141414] p-4 text-zinc-100"
    role="alertdialog"
    :aria-label="asking ? 'Rune asks' : 'Needs your OK'"
  >
    <header class="flex h-6 items-center gap-2">
      <span class="size-2 shrink-0 animate-pulse rounded-full bg-[#d71921]" />
      <span class="text-caption text-[#d71921] uppercase">{{
        asking ? 'Asks you' : 'Needs your OK'
      }}</span>
      <span class="ml-auto truncate text-caption text-zinc-500 uppercase">{{
        question.source
      }}</span>
    </header>

    <p v-if="asking" class="text-body">{{ question.text }}</p>
    <p v-else class="text-body"><span class="text-zinc-400">Wants to </span>{{ question.text }}</p>
    <p
      v-if="question.reason"
      class="line-clamp-2 text-label text-zinc-400"
      :title="question.reason"
    >
      “{{ question.reason }}”
    </p>

    <ul v-if="question.reasons.length" class="flex flex-col gap-1">
      <li
        v-for="reason in question.reasons.slice(0, 3)"
        :key="reason"
        class="flex items-start gap-2 text-label text-zinc-300"
      >
        <span class="mt-[6px] size-1 shrink-0 rounded-full bg-[#d71921]" />
        {{ reason }}
      </li>
    </ul>

    <div class="mt-1 flex gap-2">
      <button
        ref="no"
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 text-label text-zinc-200 transition hover:bg-white/10"
        @click="agent.answer(false)"
      >
        No
      </button>
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full bg-[#d71921] text-label text-white transition hover:bg-[#b9151c]"
        @click="agent.answer(true)"
      >
        {{ asking ? 'Yes' : 'Yes, allow' }}
      </button>
    </div>
    <p class="text-center text-caption text-zinc-600 uppercase">
      No answer in {{ clock }} counts as no
    </p>
  </section>
</template>