<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { useDebounceFn, useEventListener } from '@vueuse/core'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { problemText, useFlight } from '@/composables/useFlight'

import ArtifactCard from './ArtifactCard.vue'

const props = withDefaults(
  defineProps<{ agent: Agent; active?: boolean; openTask?: (id: string) => void }>(),
  { active: true },
)
const flight = useFlight()
const deleting = ref<string | null>(null)
const refreshing = ref(false)
let disposed = false
const unlisten: (() => void)[] = []
async function refresh() {
  if (disposed || props.active === false || deleting.value || refreshing.value) return
  refreshing.value = true
  try {
    await flight.loadRuns()
  } finally {
    refreshing.value = false
  }
}
const scheduleRefresh = useDebounceFn(refresh, 300, { maxWait: 1000 })
useEventListener(window, 'focus', refresh)
watch(
  () => props.active,
  (active) => {
    if (active) void refresh()
  },
)
async function remove(id: string) {
  if (deleting.value || refreshing.value) return
  deleting.value = id
  try {
    await flight.remove(id)
  } catch (error) {
    flight.problem.value = problemText(error)
  } finally {
    deleting.value = null
  }
}

onMounted(async () => {
  void refresh()
  for (const name of ['tasks_updated', 'task_done', 'analysis_updated', 'workspace_section']) {
    try {
      const off = await listen(name, () => void scheduleRefresh())
      if (disposed) off()
      else unlisten.push(off)
    } catch {
      /* Manual refresh remains available. */
    }
  }
})
onBeforeUnmount(() => {
  disposed = true
  unlisten.forEach((off) => off())
})
</script>

<template>
  <div class="flex flex-col gap-4" aria-label="Task history">
    <button
      type="button"
      class="self-end cursor-pointer text-label text-zinc-400 hover:text-zinc-100 disabled:opacity-40"
      :disabled="refreshing || deleting !== null"
      @click="refresh"
    >
      {{ refreshing ? 'Refreshing…' : 'Refresh tasks' }}
    </button>
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
        :deleting="deleting === run.id"
        :delete-disabled="deleting !== null || refreshing"
        @open="props.openTask ? props.openTask(run.id) : props.agent.openReview(run.id)"
        @remove="remove(run.id)"
      />
    </div>
  </div>
</template>