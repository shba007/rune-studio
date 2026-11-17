<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { useIntervalFn } from '@vueuse/core'
import { computed, nextTick, onMounted, ref, useTemplateRef, watch, type Ref } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { ConfirmRequestPayload } from '@/types/protocol'
import type { LiveStep } from '@/config/liveSteps'
import ToolResultView from './ToolResultView.vue'

const props = withDefaults(
  defineProps<{
    agent: {
      question: Ref<ConfirmRequestPayload | null>
      answer: Agent['answer']
      choose?: Agent['choose']
      steps?: Ref<LiveStep[]>
    }
    compact?: boolean
  }>(),
  { compact: true },
)
const agent = props.agent

const question = computed(() => agent.question.value)
const asking = computed(() => question.value?.kind === 'question')
const restarting = computed(() => question.value?.kind === 'restart')
const options = computed(() =>
  asking.value &&
  question.value?.options &&
  question.value.options.length >= 2 &&
  question.value.options.length <= 4 &&
  Number.isInteger(question.value.suggested) &&
  question.value.suggested! >= 0 &&
  question.value.suggested! < question.value.options.length
    ? question.value.options
    : null,
)
const content = computed(() =>
  asking.value && question.value?.taskId != null
    ? agent.steps?.value
        .filter(
          (step) => step.taskId === question.value?.taskId && step.kind === 'tool' && step.result,
        )
        .at(-1)?.result
    : null,
)
const left = ref(0)
watch(question, (asked) => (left.value = asked?.timeoutSecs ?? 0), { immediate: true })
useIntervalFn(() => (left.value = Math.max(0, left.value - 1)), 1000)

const clock = computed(
  () => `${Math.floor(left.value / 60)}:${String(left.value % 60).padStart(2, '0')}`,
)

const no = useTemplateRef<InstanceType<typeof RuneButton>>('no')
const optionGroup = useTemplateRef<HTMLElement>('optionGroup')
async function focusAnswer() {
  await nextTick()
  if (options.value)
    optionGroup.value?.querySelector<HTMLButtonElement>('[data-suggested="true"]')?.focus()
  else no.value?.$el.focus()
}
onMounted(focusAnswer)
watch(() => question.value?.id, focusAnswer)
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

    <ToolResultView
      v-if="content"
      :text="content.text"
      :schema="content.outputSchema"
      :structured-content="content.structuredContent"
      :compact="compact"
      :show-raw="false"
    />
    <div v-if="options" ref="optionGroup" class="mt-1 flex flex-col gap-2">
      <RuneButton
        v-for="(option, index) in options"
        :key="option.label"
        size="sm"
        :variant="index === question.suggested ? 'primary' : 'secondary'"
        class="w-full !flex-col !items-start !rounded-rune-tile !text-left normal-case"
        :data-suggested="index === question.suggested"
        @click="agent.choose?.(index)"
      >
        <span
          >{{ option.label }}
          <span v-if="index === question.suggested" class="text-rune-caption"
            >· Suggested</span
          ></span
        >
        <span class="text-rune-caption normal-case">{{ option.outcome }}</span>
      </RuneButton>
    </div>
    <div v-else class="mt-1 flex gap-2">
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