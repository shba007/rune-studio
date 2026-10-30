<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed, ref, useId, watch } from 'vue'
import type { DecisionAnswer, DecisionQuestion } from '@/types/decisions'
import { DOMAINS, domainById } from '@/config/domains'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import RuneChoice from './RuneChoice.vue'

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
const domainChoice = ref('')
const { showSanctuary } = useDomainPrefs()
const domainOptions = computed(() => [
  { value: '', label: 'Choose another domain' },
  ...DOMAINS.map((domain) => ({
    value: domain.id,
    label: `${domain.name}${domain.zone === 'sanctuary' && !showSanctuary.value ? ' · Sanctuary (hidden)' : ''}`,
  })),
])
const allowsFreeform = computed(() =>
  ['domain', 'clarification', 'goal_review', 'attachment'].includes(props.question.kind),
)
watch(
  () => [props.question.id, props.question.revision],
  () => {
    optionId.value = props.question.draft?.optionId ?? null
    text.value = props.question.draft?.text ?? ''
    domainChoice.value =
      props.question.kind === 'domain' && DOMAINS.some((d) => d.id === text.value) ? text.value : ''
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
  if (id !== null && props.question.kind === 'domain') {
    domainChoice.value = ''
    text.value = ''
  }
  optionId.value = id
  emit('draft', answer())
}
function typeAnswer(event: Event) {
  domainChoice.value = ''
  text.value = (event.target as HTMLTextAreaElement).value
  choose(null)
}
function chooseDomain(value: string) {
  domainChoice.value = value
  text.value = value
  choose(null)
}
</script>

<template>
  <form
    class="flex flex-col gap-rune-2 rounded-rune-tile bg-rune-surface p-rune-2 text-rune-ink"
    :aria-labelledby="`${group}-title`"
    @submit.prevent="valid && !busy && emit('answer', answer())"
  >
    <h2 :id="`${group}-title`" class="m-0 text-rune-body">{{ question.title }}</h2>
    <p v-if="question.explanation" class="m-0 text-rune-caption text-rune-text-3">
      {{ question.explanation }}
    </p>
    <fieldset :disabled="busy" class="m-0 flex min-w-0 flex-col gap-0.5 p-0">
      <legend class="sr-only">Choose an answer or write your own</legend>
      <label
        v-for="(option, index) in question.options"
        :key="option.id"
        class="flex cursor-pointer items-start gap-rune-2 px-rune-2 py-rune-1 text-rune-body transition-colors first:rounded-t-rune-tile has-[input:focus-visible]:ring-1 has-[input:focus-visible]:ring-white/40"
        :class="
          optionId === option.id ? 'bg-rune-ink text-black' : 'bg-rune-raised hover:bg-white/8'
        "
      >
        <input
          type="radio"
          :name="group"
          :value="option.id"
          :checked="optionId === option.id"
          class="sr-only"
          @change="choose(option.id)"
        />
        <span class="min-w-0"
          ><span class="block"
            >{{ option.label
            }}<span
              v-if="index === 0 && ['domain', 'clarification'].includes(question.kind)"
              class="ml-2 text-rune-heading uppercase opacity-60"
              >Recommended</span
            ></span
          ><span
            class="block text-rune-caption"
            :class="optionId === option.id ? 'opacity-70' : 'text-rune-text-3'"
            >{{ option.explanation }}</span
          ><span
            v-if="
              question.kind === 'domain' &&
              !showSanctuary &&
              domainById(option.id).zone === 'sanctuary'
            "
            class="block text-rune-caption"
            >Sanctuary · hidden in Workspace</span
          ></span
        >
      </label>
      <label
        v-if="allowsFreeform"
        class="flex flex-col gap-1 rounded-b-rune-tile bg-rune-raised px-rune-2 py-rune-1 text-rune-caption text-rune-text-3"
      >
        <span>Your own answer</span>
        <textarea
          :value="text"
          rows="2"
          maxlength="4000"
          class="w-full resize-y rounded-rune-tile bg-rune-sunken p-rune-1 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3"
          placeholder="Tell Rune otherwise"
          @input="typeAnswer"
        />
      </label>
    </fieldset>
    <template v-if="question.kind === 'domain'">
      <RuneChoice
        :model-value="domainChoice"
        label="Another goal domain"
        menu
        :disabled="busy"
        :options="domainOptions"
        @update:model-value="chooseDomain"
      />
      <p v-if="!showSanctuary" class="m-0 text-rune-caption text-rune-text-3">
        Sanctuary domains are hidden. You can confirm one here; turn on Show Sanctuary domains in
        Settings to display its goal and numeric tracking. Confirmation does not change Settings.
      </p>
    </template>
    <p v-if="error" role="alert" class="m-0 text-rune-caption text-error">{{ error }}</p>
    <div class="flex flex-wrap items-center gap-rune-1">
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