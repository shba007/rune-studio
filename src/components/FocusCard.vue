<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue'

import type { Focus } from '@/composables/useFocus'
import {
  clock,
  clockTime,
  durationLabel,
  nextCommitment,
  startAt,
  loggedLine,
  MIN_MINUTES,
  stepMinutes,
  variantLabel,
  type FocusSuggestion,
} from '@/config/focus'

import AppIcon from './AppIcon.vue'
import RuneButton from './RuneButton.vue'
import RuneIconButton from './RuneIconButton.vue'

const props = defineProps<{ focus: Focus }>()

const title = ref('')
const doneWhen = ref('')
const minutes = ref(props.focus.preset.value.defaultMinutes)
watch(
  () => props.focus.preset.value.id,
  () => (minutes.value = props.focus.preset.value.defaultMinutes),
)
const suggestion = ref<FocusSuggestion | null>(null)
const choice = ref(0)
const commitmentTitle = ref('')
const commitmentTime = ref('')
const commitmentMinutes = ref(30)
const commitmentStart = computed(() => startAt(commitmentTime.value, new Date()))
const canAddCommitment = computed(
  () => !!commitmentTitle.value.trim() && commitmentStart.value !== null,
)
const upcoming = computed(() =>
  nextCommitment(props.focus.commitments.value, props.focus.now.value),
)
const tooLong = computed(
  () => props.focus.free.value !== null && minutes.value * 60_000 > props.focus.free.value,
)

async function addCommitment() {
  const start = commitmentStart.value
  if (!canAddCommitment.value || start === null) return
  if (await props.focus.addCommitment(commitmentTitle.value, start, commitmentMinutes.value)) {
    commitmentTitle.value = ''
    commitmentTime.value = ''
    commitmentMinutes.value = 30
  }
}

const titleField = useTemplateRef<HTMLInputElement>('titleField')
const doneButton = useTemplateRef<{ $el: HTMLElement }>('doneButton')

watch(
  () => props.focus.hotkeyTick.value,
  async () => {
    await nextTick()
    if (props.focus.active.value) doneButton.value?.$el.focus()
    else titleField.value?.focus()
  },
)

watch(
  () => props.focus.dictated.value,
  async (heard) => {
    if (!heard || !props.focus.dictating.value) return
    title.value = heard.text.trim().slice(0, 120)
    await props.focus.setDictation(false)
    await suggest()
  },
)
watch(
  () => props.focus.formOpen.value && !props.focus.active.value,
  (open) => {
    if (!open && props.focus.dictating.value) void props.focus.setDictation(false)
  },
)
onBeforeUnmount(() => {
  if (props.focus.dictating.value) void props.focus.setDictation(false)
})

watch(
  () => props.focus.formOpen.value,
  async (open) => {
    if (!open) return
    await nextTick()
    titleField.value?.focus()
  },
  { immediate: true },
)

function apply(index: number) {
  const option = suggestion.value?.options[index]
  if (!option) return
  choice.value = index
  title.value = option.title
  doneWhen.value = option.doneWhen
  minutes.value = option.minutes
}

const goal = ref('')

async function suggest() {
  if (!title.value.trim()) return
  const untouched = suggestion.value?.options[choice.value]?.title === title.value
  if (!untouched || !goal.value) goal.value = title.value
  const result = await props.focus.suggest(goal.value)
  if (!result?.options.length) return
  suggestion.value = result
  apply(0)
}

const another = () => apply((choice.value + 1) % (suggestion.value?.options.length ?? 1))

async function start() {
  if (!title.value.trim()) return
  if (await props.focus.start(title.value, doneWhen.value, minutes.value)) {
    title.value = ''
    doneWhen.value = ''
    minutes.value = props.focus.preset.value.defaultMinutes
    suggestion.value = null
    choice.value = 0
    goal.value = ''
  }
}

const field =
  'h-10 w-full min-w-0 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3'
</script>

<template>
  <section
    v-if="focus.active.value && focus.block.value"
    aria-label="Focus block"
    class="flex flex-col gap-rune-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-body text-rune-ink"
    @keydown.ctrl.enter.prevent="focus.finish('done')"
    @keydown.ctrl.e.prevent="focus.over.value || focus.wrapUp.value ? focus.extend() : undefined"
  >
    <header class="flex items-center justify-between">
      <span class="text-rune-caption text-rune-text-2 uppercase">{{
        focus.over.value ? "Time's up" : 'Focusing'
      }}</span>
    </header>
    <p
      role="timer"
      aria-label="Time left in this focus block"
      class="m-0 text-center text-[40px] leading-none font-semibold tabular-nums"
    >
      {{ clock(focus.remaining.value) }}
    </p>
    <p class="m-0 leading-relaxed">{{ focus.block.value.title }}</p>
    <p v-if="focus.block.value.doneWhen" class="m-0 text-rune-caption text-rune-text-2">
      {{ focus.preset.value.doneWhenLabel }}: {{ focus.block.value.doneWhen }}
    </p>
    <p
      v-if="focus.block.value.handedOffMs !== null"
      role="status"
      aria-label="Handed to the agent"
      class="m-0 text-rune-caption text-rune-text-2"
    >
      With the agent. Review its plan in your requests; it will ask before anything it cannot undo.
    </p>
    <p v-if="focus.wrapUp.value" role="status" aria-label="Wrap up" class="m-0 text-rune-caption">
      Wrap up: {{ focus.wrapUp.value.title }} starts at {{ clockTime(focus.wrapUp.value.startMs) }}
    </p>
    <p v-if="focus.error.value" role="alert" class="m-0 text-rune-caption text-error">
      {{ focus.error.value }}
    </p>
    <RuneButton
      v-if="focus.wrapUp.value || focus.over.value"
      variant="ghost"
      :disabled="focus.pending.value"
      @click="focus.extend()"
    >
      Extend 5m
    </RuneButton>
    <RuneButton
      v-if="focus.block.value.handedOffMs === null"
      variant="ghost"
      :disabled="focus.pending.value"
      @click="focus.handOff()"
    >
      Hand to agent
    </RuneButton>
    <div class="flex gap-2">
      <RuneButton class="flex-1" :disabled="focus.pending.value" @click="focus.finish('stopped')">
        Stop
      </RuneButton>
      <RuneButton
        ref="doneButton"
        variant="primary"
        class="flex-1"
        :disabled="focus.pending.value"
        @click="focus.finish('done')"
      >
        Done
      </RuneButton>
    </div>
  </section>

  <form
    v-else
    aria-label="Start a focus block"
    class="flex flex-col gap-rune-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-body text-rune-ink"
    @submit.prevent="start"
    @keydown.ctrl.enter.prevent="suggest"
  >
    <header class="flex items-center justify-between">
      <span class="text-rune-caption text-rune-text-2 uppercase">Focus</span>
      <RuneIconButton variant="ghost" size="xs" label="Close focus" @click="focus.closeForm()">
        ×
      </RuneIconButton>
    </header>
    <div class="flex items-center gap-2">
      <input
        ref="titleField"
        v-model="title"
        type="text"
        maxlength="120"
        aria-label="What will you work on?"
        placeholder="What will you work on?"
        autocomplete="off"
        spellcheck="false"
        :class="field"
      />
      <RuneIconButton
        :label="focus.dictating.value ? 'Stop listening' : 'Say what you will work on'"
        :aria-pressed="focus.dictating.value"
        variant="ghost"
        size="sm"
        :disabled="focus.pending.value"
        @click="focus.setDictation(!focus.dictating.value)"
      >
        <AppIcon name="voice-input" size="sm" />
      </RuneIconButton>
    </div>
    <input
      v-model="doneWhen"
      type="text"
      maxlength="200"
      :aria-label="focus.preset.value.doneWhenLabel"
      :placeholder="`${focus.preset.value.doneWhenLabel}… (optional)`"
      autocomplete="off"
      spellcheck="false"
      :class="field"
    />
    <div class="flex items-center justify-between gap-2">
      <RuneIconButton
        label="5 minutes shorter"
        size="sm"
        :disabled="minutes <= MIN_MINUTES"
        @click="minutes = stepMinutes(minutes, -1)"
      >
        −
      </RuneIconButton>
      <span class="tabular-nums" role="status" aria-label="Block length">{{ minutes }} min</span>
      <RuneIconButton
        label="5 minutes longer"
        size="sm"
        :disabled="minutes >= focus.preset.value.capMinutes"
        @click="minutes = stepMinutes(minutes, 1, focus.preset.value.capMinutes)"
      >
        +
      </RuneIconButton>
    </div>
    <p
      v-if="focus.free.value !== null && upcoming"
      role="status"
      aria-label="Free time"
      class="m-0 text-rune-caption text-rune-text-2"
    >
      Free: {{ durationLabel(focus.free.value) }} until {{ upcoming.title }}
      <template v-if="tooLong"> · this block is longer than your free time</template>
    </p>
    <div v-if="suggestion" class="flex items-center justify-between gap-2">
      <p role="status" aria-label="Suggestion" class="m-0 text-rune-caption text-rune-text-2">
        <template v-if="suggestion.source === 'model'">
          {{ variantLabel(suggestion.options, choice) }} ({{ choice + 1 }} of
          {{ suggestion.options.length }})
        </template>
        <template v-else>{{ suggestion.note }}</template>
      </p>
      <RuneButton v-if="suggestion.options.length > 1" size="sm" variant="ghost" @click="another">
        Another
      </RuneButton>
    </div>
    <p v-if="focus.error.value" role="alert" class="m-0 text-rune-caption text-error">
      {{ focus.error.value }}
    </p>
    <RuneButton
      :disabled="!title.trim() || focus.suggesting.value || focus.pending.value"
      @click="suggest"
    >
      {{ focus.suggesting.value ? 'Thinking…' : 'Suggest a block' }}
    </RuneButton>
    <RuneButton type="submit" variant="primary" :disabled="!title.trim() || focus.pending.value">
      Start focus
    </RuneButton>
    <details class="text-rune-caption text-rune-text-2">
      <summary class="cursor-pointer uppercase">
        Commitments ({{ focus.commitments.value.length }})
      </summary>
      <ul
        v-if="focus.commitments.value.length"
        aria-label="Upcoming commitments"
        class="m-0 mt-2 flex list-none flex-col gap-1 p-0"
      >
        <li
          v-for="item in focus.commitments.value"
          :key="item.id"
          class="flex items-center justify-between gap-2"
        >
          <span class="truncate">{{ clockTime(item.startMs) }} · {{ item.title }}</span>
          <span v-if="item.source === 'calendar'" class="shrink-0 text-rune-text-3">calendar</span>
          <RuneIconButton
            v-else
            variant="ghost"
            size="xs"
            :label="`Remove ${item.title}`"
            :disabled="focus.pending.value"
            @click="focus.removeCommitment(item.id)"
          >
            ×
          </RuneIconButton>
        </li>
      </ul>
      <div class="mt-2 flex flex-wrap items-center gap-2">
        <RuneButton size="sm" :disabled="focus.pending.value" @click="focus.syncCalendar()">
          Sync calendar
        </RuneButton>
        <span
          v-if="focus.calendar.value"
          role="status"
          aria-label="Calendar sync"
          class="min-w-0 flex-1"
        >
          {{ focus.calendar.value.message }}
        </span>
      </div>
      <div class="mt-2 flex flex-col gap-2">
        <input
          v-model="commitmentTitle"
          type="text"
          maxlength="120"
          aria-label="Commitment"
          placeholder="Commitment, e.g. Standup"
          autocomplete="off"
          :class="field"
          @keydown.enter.prevent="addCommitment"
        />
        <div class="flex items-center gap-2">
          <input
            v-model="commitmentTime"
            type="time"
            aria-label="Commitment starts at"
            :class="field"
            @keydown.enter.prevent="addCommitment"
          />
          <input
            v-model.number="commitmentMinutes"
            type="number"
            min="1"
            max="720"
            aria-label="Commitment length in minutes"
            :class="[field, 'w-24']"
            @keydown.enter.prevent="addCommitment"
          />
        </div>
        <RuneButton
          size="sm"
          :disabled="!canAddCommitment || focus.pending.value"
          @click="addCommitment"
        >
          Add commitment
        </RuneButton>
      </div>
    </details>
    <ul
      v-if="focus.snapshot.value.recent.length"
      aria-label="Recent focus blocks"
      class="m-0 flex list-none flex-col gap-1 p-0 text-rune-caption text-rune-text-2"
    >
      <li v-for="item in focus.snapshot.value.recent.slice(0, 3)" :key="item.id" class="truncate">
        {{ loggedLine(item) }}
      </li>
    </ul>
  </form>
</template>