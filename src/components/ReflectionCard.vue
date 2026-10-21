<script setup lang="ts">
import { useIntervalFn, useNow } from '@vueuse/core'
import { computed, onMounted, useTemplateRef } from 'vue'

import { clock, secondsLeft } from '@/config/reflection'
import type { Agent } from '@/composables/useAgent'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const now = useNow({ scheduler: (update) => useIntervalFn(update, 250) })
const left = computed(() => {
  const ends = agent.reflectionPrompt.value?.endsAt
  return ends === undefined ? 0 : secondsLeft(now.value.getTime(), ends)
})

const deny = useTemplateRef<HTMLButtonElement>('deny')
onMounted(() => deny.value?.focus())
</script>

<template>
  <section
    v-if="agent.reflectionPrompt.value"
    class="flex w-full flex-col gap-2 rounded-[28px] border border-white/15 bg-[#141414] p-4 text-zinc-100"
    role="alertdialog"
    aria-label="Review recent tasks"
  >
    <header class="flex h-6 items-center gap-2">
      <span class="size-2 shrink-0 rounded-full bg-[#f1f1ef]" />
      <span class="text-caption text-zinc-300 uppercase">Rune</span>
      <span class="ml-auto text-caption text-zinc-500 uppercase" aria-live="off">
        Starts in {{ clock(left) }}
      </span>
    </header>
    <p class="text-body">Rune wants to review its recent tasks.</p>
    <div class="mt-1 flex gap-2">
      <button
        ref="deny"
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 text-label text-zinc-200 uppercase transition hover:bg-white/10"
        @click="agent.answerReflection(false)"
      >
        Deny
      </button>
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full bg-[#f1f1ef] text-label text-[#111] uppercase transition hover:bg-white"
        @click="agent.answerReflection(true)"
      >
        Start
      </button>
    </div>
  </section>
</template>