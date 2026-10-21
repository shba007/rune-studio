<script setup lang="ts">
import { onMounted } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { useFlight } from '@/composables/useFlight'

import AppIcon from './AppIcon.vue'
import ArtifactCard from './ArtifactCard.vue'
import PageTabs from './PageTabs.vue'

/**
 * The recorded sessions, newest first. Choosing one opens it in Rune Preview, the window that
 * replays it. Nothing here changes the record.
 */
const props = defineProps<{ agent: Agent }>()
const agent = props.agent
const flight = useFlight()

onMounted(() => flight.loadRuns())
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

    <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-3 pb-4">
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
      <ArtifactCard
        v-for="run in flight.runs.value"
        :key="run.id"
        :run="run"
        @open="agent.openReview(run.id)"
      />
    </div>
  </section>
</template>