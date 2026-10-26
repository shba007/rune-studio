<script setup lang="ts">
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
    : text.value.trim().length > 0,
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
    class="flex flex-col gap-3 rounded-3xl border border-white/10 bg-[#141414] p-4 text-zinc-100"
    :aria-labelledby="`${group}-title`"
    @submit.prevent="valid && !busy && emit('answer', answer())"
  >
    <h2 :id="`${group}-title`" class="m-0 text-body">{{ question.title }}</h2>
    <p v-if="question.explanation" class="m-0 text-label text-zinc-400">
      {{ question.explanation }}
    </p>
    <fieldset :disabled="busy" class="m-0 flex min-w-0 flex-col gap-2 border-0 p-0">
      <legend class="sr-only">Choose an answer or write your own</legend>
      <label
        v-for="(option, index) in question.options"
        :key="option.id"
        class="flex cursor-pointer items-start gap-2 rounded-2xl border border-white/10 p-3 text-label"
        :class="optionId === option.id ? 'bg-white/10' : 'hover:bg-white/5'"
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
              v-if="index === 0 && !['consent', 'attachment'].includes(question.kind)"
              class="ml-2 text-caption text-zinc-400"
              >Recommended</span
            ></span
          ><span class="block text-caption text-zinc-400">{{ option.explanation }}</span></span
        >
      </label>
      <label class="flex flex-col gap-2 rounded-2xl border border-white/10 p-3 text-label">
        <span>Your own answer</span>
        <textarea
          :value="text"
          rows="2"
          maxlength="4000"
          class="w-full resize-y rounded-xl bg-white/5 p-2 text-zinc-100 placeholder:text-zinc-400 outline-none focus:ring-1 focus:ring-white/30"
          placeholder="Tell Rune otherwise"
          @input="typeAnswer"
        />
      </label>
    </fieldset>
    <p v-if="error" role="alert" class="m-0 text-label text-[#ff6b72]">{{ error }}</p>
    <div class="flex gap-2">
      <button
        v-if="deferable"
        type="button"
        :disabled="busy"
        class="min-h-9 cursor-pointer rounded-full px-3 text-label text-zinc-400 disabled:opacity-40"
        @click="emit('defer', question.id)"
      >
        Decide later
      </button>
      <button
        type="button"
        :disabled="busy"
        class="min-h-9 cursor-pointer rounded-full px-3 text-label text-zinc-400 disabled:opacity-40"
        @click="emit('answer', { ...answer(), optionId: 'cancel' })"
      >
        {{ cancelLabel }}
      </button>
      <button
        type="submit"
        :disabled="!valid || busy"
        class="ml-auto min-h-9 cursor-pointer rounded-full bg-zinc-100 px-4 text-label text-zinc-950 disabled:opacity-40"
      >
        {{ busy ? 'Saving…' : 'Continue' }}
      </button>
    </div>
  </form>
</template>