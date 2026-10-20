<script setup lang="ts">
import { onMounted } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { useFlight } from '@/composables/useFlight'
import { lengthText, stepDots, toneOf, whenText } from '@/config/artifacts'

import AppIcon from './AppIcon.vue'
import PageTabs from './PageTabs.vue'

/**
 * The recorded sessions, newest first. Choosing one opens it in Rune Preview, the window that
 * replays it. Nothing here changes the record.
 */
const props = defineProps<{ agent: Agent }>()
const agent = props.agent
const flight = useFlight()

onMounted(() => flight.loadRuns())

const TONE = {
  ok: 'bg-[#f1f1ef]',
  bad: 'bg-[#d71921]',
  stopped: 'border border-zinc-400',
  open: 'border border-dashed border-zinc-400',
} as const
</script>

<template>
  <section
    class="flex h-full w-full flex-col overflow-hidden rounded-[28px] border border-white/8 bg-[#141414] text-zinc-100"
  >
    <header class="flex items-center gap-1 px-4 pt-4 pb-2">
      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full text-zinc-300 transition hover:bg-white/8 hover:text-white"
        aria-label="Back"
        @click="agent.setMode(agent.home())"
      >
        <AppIcon name="back" />
      </button>
      <div class="flex min-w-0 flex-1 flex-col pl-2">
        <h2 class="text-title">Artifacts</h2>
        <span class="text-caption text-zinc-500 uppercase"
          >{{ flight.runs.value.length }} recorded</span
        >
      </div>
      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full text-zinc-300 transition hover:bg-white/8 hover:text-white"
        aria-label="Reload the list"
        @click="flight.loadRuns()"
      >
        <AppIcon name="refresh" />
      </button>
    </header>
    <PageTabs :agent="agent" current="artifacts" />

    <div class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-3 pb-4">
      <p v-if="flight.problem.value" class="m-0 px-2 py-4 text-label text-zinc-400" role="alert">
        {{ flight.problem.value }}
      </p>
      <p
        v-else-if="flight.runsLoaded.value && flight.runs.value.length === 0"
        class="m-0 px-2 py-6 text-center text-label text-zinc-500"
        data-testid="empty"
      >
        Nothing recorded yet.<br />Run a task and it shows up here.
      </p>
      <button
        v-for="run in flight.runs.value"
        :key="run.id"
        type="button"
        class="flex w-full cursor-pointer flex-col gap-2 rounded-2xl border-0 bg-transparent p-3 text-left text-zinc-100 transition hover:bg-white/6"
        data-testid="artifact"
        @click="agent.openReview(run.id)"
      >
        <span class="flex items-start gap-2">
          <span
            class="mt-1.5 size-2 shrink-0 rounded-full"
            :class="TONE[toneOf(run.outcome)]"
            :title="run.outcome ?? 'open'"
          />
          <span class="line-clamp-2 min-w-0 flex-1 text-body leading-snug">{{ run.task }}</span>
          <AppIcon name="play" class="mt-1 shrink-0 text-zinc-500" />
        </span>
        <span
          class="flex items-center gap-2 pl-4 text-caption whitespace-nowrap text-zinc-500 uppercase"
        >
          <span class="flex items-center gap-0.5" aria-hidden="true">
            <span
              v-for="n in stepDots(run.steps).count"
              :key="n"
              class="size-1 rounded-full bg-zinc-400"
            />
            <span v-if="stepDots(run.steps).more" class="pl-0.5 leading-none">+</span>
          </span>
          <span>{{ run.steps }} {{ run.steps === 1 ? 'step' : 'steps' }}</span>
          <span v-if="lengthText(run)">· {{ lengthText(run) }}</span>
        </span>
        <span class="pl-4 text-caption text-zinc-600 uppercase">{{
          whenText(run.startedUtc)
        }}</span>
      </button>
    </div>
  </section>
</template>