<script setup lang="ts">
import RuneIconButton from './RuneIconButton.vue'
import { useIntersectionObserver } from '@vueuse/core'
import { ref, useTemplateRef } from 'vue'
import { invoke } from '@tauri-apps/api/core'

import { problemText, useFlight } from '@/composables/useFlight'
import { lengthText, toneOf } from '@/config/artifacts'
import type { FlightRun } from '@/config/flight'

import AppIcon from './AppIcon.vue'
import WhenText from './WhenText.vue'

const props = defineProps<{ run: FlightRun; deleting?: boolean; deleteDisabled?: boolean }>()
defineEmits<{ open: []; remove: [] }>()

const flight = useFlight()
const card = useTemplateRef<HTMLElement>('card')
const picture = ref<string | null>(null)
const failed = ref(false)
const copying = ref(false)
const notice = ref<string | null>(null)
async function copyPrompt() {
  if (copying.value) return
  copying.value = true
  notice.value = null
  try {
    const prompt = await invoke<string>('flight_original_prompt', { id: props.run.id })
    await navigator.clipboard.writeText(prompt)
    notice.value = 'Prompt copied'
  } catch (error) {
    notice.value = problemText(error)
  } finally {
    copying.value = false
  }
}

const { stop } = useIntersectionObserver(card, async ([entry]) => {
  if (!entry?.isIntersecting) return
  stop()
  picture.value = await flight.fileOf(props.run.thumb, props.run.thumbRun ?? props.run.id)
  failed.value = picture.value === null
})

const TONE = {
  ok: 'bg-rune-ink',
  bad: 'bg-attention',
  stopped: 'bg-rune-raised',
  open: 'bg-rune-raised',
} as const
</script>

<template>
  <article
    ref="card"
    class="group flex w-full shrink-0 flex-col overflow-hidden rounded-rune-tile bg-rune-sunken text-left text-rune-ink transition"
    data-testid="artifact"
    :data-task-id="run.id"
  >
    <button
      type="button"
      class="flex w-full cursor-pointer flex-col bg-transparent p-0 text-left text-inherit disabled:opacity-50"
      :disabled="deleting"
      :aria-label="`Open task: ${run.task}`"
      @click="$emit('open')"
    >
      <span class="relative block aspect-video w-full overflow-hidden bg-black">
        <img
          v-if="picture"
          :src="picture"
          alt=""
          loading="lazy"
          class="size-full object-cover transition duration-300 group-hover:scale-[1.03]"
          @error="failed = true"
        />
        <span
          v-if="!picture || failed"
          class="absolute inset-0 bg-[radial-gradient(circle,rgb(255_255_255/0.18)_1px,transparent_1.5px)] bg-size-[8px_8px]"
          aria-hidden="true"
        />
        <span
          class="absolute right-2 bottom-2 grid size-8 place-items-center rounded-full bg-black/60 text-rune-ink opacity-0 backdrop-blur transition group-hover:opacity-100"
        >
          <AppIcon name="play" />
        </span>
        <span
          class="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-black/60 px-2 py-0.5 text-rune-heading text-rune-ink uppercase backdrop-blur"
        >
          <span class="size-1.5 rounded-full" :class="TONE[toneOf(run.outcome)]" />
          {{ run.outcome ?? 'open' }}
        </span>
      </span>
      <span class="flex flex-col gap-1 p-3">
        <span class="line-clamp-2 text-rune-body leading-snug">{{ run.task }}</span>
        <span
          class="flex items-center gap-2 text-rune-heading whitespace-nowrap text-rune-text-2 uppercase"
        >
          <span>{{ run.steps }} {{ run.steps === 1 ? 'step' : 'steps' }}</span>
          <span v-if="lengthText(run)">· {{ lengthText(run) }}</span>
          <span class="ml-auto"><WhenText :iso="run.startedUtc" relative /></span>
        </span>
      </span>
    </button>
    <div class="flex items-center gap-1 px-3 py-1">
      <p
        class="m-0 min-w-0 flex-1 text-rune-caption text-rune-text-2"
        role="status"
        aria-live="polite"
      >
        {{ notice }}
      </p>
      <RuneIconButton
        variant="ghost"
        size="sm"
        label="Copy original prompt"
        title="Copy original prompt"
        :disabled="copying || deleting"
        @click="copyPrompt"
      >
        <AppIcon name="copy" size="sm" />
      </RuneIconButton>
      <RuneIconButton
        variant="ghost"
        size="sm"
        label="Delete task"
        :title="
          run.active
            ? 'Finish or stop this task before deleting it'
            : 'Delete task and its saved files'
        "
        :disabled="run.active || deleting || deleteDisabled"
        @click="$emit('remove')"
      >
        <AppIcon name="trash" size="sm" />
      </RuneIconButton>
    </div>
  </article>
</template>