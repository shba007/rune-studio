<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core'
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const question = computed(() => agent.question.value)
const asking = computed(() => question.value?.kind === 'question')
const restarting = computed(() => question.value?.kind === 'restart')
const left = ref(0)
watch(question, (asked) => (left.value = asked?.timeoutSecs ?? 0), { immediate: true })
useIntervalFn(() => (left.value = Math.max(0, left.value - 1)), 1000)

const clock = computed(
  () => `${Math.floor(left.value / 60)}:${String(left.value % 60).padStart(2, '0')}`,
)

const no = useTemplateRef<HTMLButtonElement>('no')
onMounted(() => no.value?.focus())
</script>

<template>
  <section
    v-if="question"
    class="flex w-full flex-col gap-2 rounded-[28px] border bg-[#141414] p-4 text-zinc-100"
    :class="asking ? 'border-white/15' : 'border-[#d71921]/50'"
    role="alertdialog"
    :aria-label="asking ? 'Rune asks' : 'Needs your OK'"
  >
    <header class="flex h-6 items-center gap-2">
      <span
        class="size-2 shrink-0 animate-pulse rounded-full"
        :class="asking ? 'bg-[#f1f1ef]' : 'bg-[#d71921]'"
      />
      <span class="text-caption uppercase" :class="asking ? 'text-zinc-300' : 'text-[#d71921]'">{{
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
        {{ restarting ? 'Keep stopped' : 'No' }}
      </button>
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full text-label transition"
        :class="
          asking
            ? 'bg-[#f1f1ef] text-[#111] hover:bg-white'
            : 'bg-[#d71921] text-white hover:bg-[#b9151c]'
        "
        @click="agent.answer(true)"
      >
        {{ restarting ? 'Restart task' : asking ? 'Yes' : 'Yes, allow' }}
      </button>
    </div>
    <p class="text-center text-caption text-zinc-500 uppercase">
      No answer in {{ clock }} counts as no
    </p>
  </section>
</template>