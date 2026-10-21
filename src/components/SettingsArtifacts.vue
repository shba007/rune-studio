<script setup lang="ts">
import { onMounted } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { useFlight } from '@/composables/useFlight'

import ArtifactCard from './ArtifactCard.vue'

const props = defineProps<{ agent: Agent }>()
const flight = useFlight()

onMounted(() => flight.loadRuns())
</script>

<template>
  <div class="flex flex-col gap-4">
    <p v-if="flight.problem.value" class="m-0 text-label text-zinc-400" role="alert">
      {{ flight.problem.value }}
    </p>
    <p
      v-else-if="flight.runsLoaded.value && flight.runs.value.length === 0"
      class="m-0 py-10 text-center text-label text-zinc-500"
      data-testid="empty"
    >
      Nothing recorded yet. Run a task and it shows up here.
    </p>
    <div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
      <ArtifactCard
        v-for="run in flight.runs.value"
        :key="run.id"
        :run="run"
        @open="props.agent.openReview(run.id)"
      />
    </div>
  </div>
</template>