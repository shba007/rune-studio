<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed, ref, useId, watch } from 'vue'
import type { DecisionAnswer, DecisionQuestion } from '@/types/decisions'

const props = withDefaults(
  defineProps<{
    question: DecisionQuestion
    busy?: boolean
    error?: string
    deferable?: boolean
    cancelLabel?: string
  }>(),
  { deferable: true, cancelLabel: 'Cancel request' },
)
const emit = defineEmits<{
  answer: [DecisionAnswer]
  draft: [DecisionAnswer]
  defer: [id: string]
}>()
const group = useId()
const optionId = ref<string | null>(null)
const text = ref('')
const allowsFreeform = computed(() =>
  ['clarification', 'goal_review', 'attachment'].includes(props.question.kind),
)
watch(
  () => [props.question.id, props.question.revision],
  () => {
    optionId.value = props.question.draft?.optionId ?? null
    text.value = props.question.draft?.text ?? ''
  },
  { immediate: true },
)
const answer = (): DecisionAnswer => ({
  id: props.question.id,
  revision: props.question.revision,
  optionId: optionId.value,
  text: text.value.trim(),
})
const valid = computed(() =>
  optionId.value !== null
    ? props.question.options.some((option) => option.id === optionId.value)
    : allowsFreeform.value && text.value.trim().length > 0,
)
function choose(id: string | null) {
  optionId.value = id
  emit('draft', answer())
}
function typeAnswer(event: Event) {
  text.value = (event.target as HTMLTextAreaElement).value
  choose(null)
}
</script>

<template>
  <form
    class="flex flex-col gap-3 rounded-rune-tile bg-rune-surface p-4 text-rune-ink"
    :aria-labelledby="`${group}-title`"
    @submit.prevent="valid && !busy && emit('answer', answer())"
  >
    <h2 :id="`${group}-title`" class="m-0 text-rune-body">{{ question.title }}</h2>
    <p v-if="question.explanation" class="m-0 text-rune-caption text-rune-text-2">
      {{ question.explanation }}
    </p>
    <fieldset :disabled="busy" class="m-0 flex min-w-0 flex-col gap-2 p-0">
      <legend class="sr-only">Choose an answer or write your own</legend>
      <label
        v-for="(option, index) in question.options"
        :key="option.id"
        class="flex cursor-pointer items-start gap-2 rounded-rune-tile p-3 text-rune-caption bg-rune-raised"
        :class="optionId === option.id ? 'bg-rune-raised' : 'hover:bg-white/5'"
      >
        <input
          type="radio"
          :name="group"
          :value="option.id"
          :checked="optionId === option.id"
          class="mt-1 accent-white"
          @change="choose(option.id)"
        />
        <span class="min-w-0"
          ><span class="block"
            >{{ option.label
            }}<span
              v-if="index === 0 && question.kind === 'clarification'"
              class="ml-2 text-rune-heading text-rune-text-2"
              >Recommended</span
            ></span
          ><span class="block text-rune-heading text-rune-text-2">{{
            option.explanation
          }}</span></span
        >
      </label>
      <label
        v-if="allowsFreeform"
        class="flex flex-col gap-2 rounded-rune-tile p-3 text-rune-caption bg-rune-raised"
      >
        <span>Your own answer</span>
        <textarea
          :value="text"
          rows="2"
          maxlength="4000"
          class="w-full resize-y rounded-rune-tile bg-rune-surface p-2 text-rune-ink placeholder:text-rune-text-3"
          placeholder="Tell Rune otherwise"
          @input="typeAnswer"
        />
      </label>
    </fieldset>
    <p v-if="error" role="alert" class="m-0 text-rune-caption text-error">{{ error }}</p>
    <div class="flex flex-wrap gap-2">
      <RuneButton
        variant="ghost"
        v-if="deferable"
        :disabled="busy"
        class="whitespace-nowrap"
        @click="emit('defer', question.id)"
      >
        Decide later
      </RuneButton>
      <RuneButton
        variant="ghost"
        :disabled="busy"
        class="whitespace-nowrap"
        @click="emit('answer', { ...answer(), optionId: 'cancel' })"
      >
        {{ cancelLabel }}
      </RuneButton>
      <RuneButton
        variant="primary"
        type="submit"
        :disabled="!valid || busy"
        class="ml-auto whitespace-nowrap"
      >
        {{ busy ? 'Saving…' : 'Continue' }}
      </RuneButton>
    </div>
  </form>
</template>