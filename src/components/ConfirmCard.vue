<script setup lang="ts">
import RuneButton from './RuneButton.vue'
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

const no = useTemplateRef<InstanceType<typeof RuneButton>>('no')
onMounted(() => no.value?.$el.focus())
</script>

<template>
  <section
    v-if="question"
    class="flex w-full flex-col gap-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-ink"
    :class="asking ? '' : ''"
    role="alertdialog"
    :aria-label="asking ? 'Rune asks' : 'Needs your OK'"
  >
    <header class="flex h-6 items-center gap-2">
      <span
        class="size-2 shrink-0 motion-safe:animate-pulse rounded-full"
        :class="asking ? 'bg-primary' : 'bg-attention'"
      />
      <span
        class="text-rune-caption uppercase"
        :class="asking ? 'text-rune-text-2' : 'text-attention'"
        >{{ asking ? 'Asks you' : 'Needs your OK' }}</span
      >
      <span class="ml-auto truncate text-rune-caption text-rune-text-2 uppercase">{{
        question.source
      }}</span>
    </header>

    <p v-if="asking" class="text-rune-body">{{ question.text }}</p>
    <p v-else class="text-rune-body">
      <span class="text-rune-text-2">Wants to </span>{{ question.text }}
    </p>
    <p
      v-if="question.reason"
      class="line-clamp-2 text-rune-caption text-rune-text-2"
      :title="question.reason"
    >
      “{{ question.reason }}”
    </p>

    <ul v-if="question.reasons.length" class="flex flex-col gap-1">
      <li
        v-for="reason in question.reasons.slice(0, 3)"
        :key="reason"
        class="flex items-start gap-2 text-rune-caption text-rune-text-2"
      >
        <span class="mt-[6px] size-1 shrink-0 rounded-full bg-attention" />
        {{ reason }}
      </li>
    </ul>

    <div class="mt-1 flex gap-2">
      <RuneButton size="sm" ref="no" class="flex-1" @click="agent.answer(false)">
        {{ restarting ? 'Keep stopped' : 'No' }}
      </RuneButton>
      <RuneButton
        size="sm"
        class="flex-1"
        :variant="asking ? 'primary' : 'danger'"
        @click="agent.answer(true)"
      >
        {{ restarting ? 'Restart task' : asking ? 'Yes' : 'Yes, allow' }}
      </RuneButton>
    </div>
    <p class="text-center text-rune-caption text-rune-text-2 uppercase">
      No answer in {{ clock }} counts as no
    </p>
  </section>
</template>