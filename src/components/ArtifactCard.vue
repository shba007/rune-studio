<script setup lang="ts">
import { useIntersectionObserver } from '@vueuse/core'
import { ref, useTemplateRef } from 'vue'

import { useFlight } from '@/composables/useFlight'
import { lengthText, toneOf } from '@/config/artifacts'
import type { FlightRun } from '@/config/flight'

import AppIcon from './AppIcon.vue'
import WhenText from './WhenText.vue'

const props = defineProps<{ run: FlightRun }>()
defineEmits<{ open: [] }>()

const flight = useFlight()
const card = useTemplateRef<HTMLElement>('card')
const picture = ref<string | null>(null)
const failed = ref(false)

const { stop } = useIntersectionObserver(card, async ([entry]) => {
  if (!entry?.isIntersecting) return
  stop()
  picture.value = await flight.fileOf(props.run.thumb, props.run.id)
  failed.value = picture.value === null
})

const TONE = {
  ok: 'bg-[#f1f1ef]',
  bad: 'bg-[#d71921]',
  stopped: 'border border-zinc-400',
  open: 'border border-dashed border-zinc-400',
} as const
</script>

<template>
  <button
    ref="card"
    type="button"
    class="group flex w-full cursor-pointer flex-col overflow-hidden rounded-3xl border border-white/10 bg-[#101010] p-0 text-left text-zinc-100 transition hover:border-white/30"
    data-testid="artifact"
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
        class="absolute right-2 bottom-2 grid size-8 place-items-center rounded-full bg-black/60 text-zinc-100 opacity-0 backdrop-blur transition group-hover:opacity-100"
      >
        <AppIcon name="play" />
      </span>
      <span
        class="absolute top-2 left-2 flex items-center gap-1 rounded-full bg-black/60 px-2 py-0.5 text-caption text-zinc-100 uppercase backdrop-blur"
      >
        <span class="size-1.5 rounded-full" :class="TONE[toneOf(run.outcome)]" />
        {{ run.outcome ?? 'open' }}
      </span>
    </span>
    <span class="flex flex-col gap-1 p-3">
      <span class="line-clamp-2 text-body leading-snug">{{ run.task }}</span>
      <span class="flex items-center gap-2 text-caption whitespace-nowrap text-zinc-500 uppercase">
        <span>{{ run.steps }} {{ run.steps === 1 ? 'step' : 'steps' }}</span>
        <span v-if="lengthText(run)">· {{ lengthText(run) }}</span>
        <span class="ml-auto"><WhenText :iso="run.startedUtc" relative /></span>
      </span>
    </span>
  </button>
</template>