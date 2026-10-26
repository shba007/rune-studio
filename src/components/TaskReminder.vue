<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { Agent } from '@/composables/useAgent'
import { useWorkspaceTasks } from '@/composables/useWorkspaceTasks'
import WhenText from './WhenText.vue'
const props = defineProps<{ agent: Agent }>()
const workspace = useWorkspaceTasks()
const deferred = ref(new Set<string>())
const task = computed(() =>
  workspace.due.value.find(
    (t) =>
      !deferred.value.has(t.occurrences.at(-1)!.id) &&
      !(t.executionMode === 'analysis' && props.agent.mode.value === 'analysis'),
  ),
)
watch(
  () => task.value?.occurrences.at(-1)?.id,
  (id) => {
    if (id && props.agent.mode.value === 'ambient') void props.agent.setMode('open')
  },
)
</script>
<template>
  <section
    v-if="task"
    aria-label="Scheduled task reminder"
    class="flex flex-col gap-3 rounded-2xl border border-white/10 bg-[#141414] p-4 text-label text-zinc-200"
  >
    <header class="flex items-center justify-between">
      <span class="text-caption text-zinc-500 uppercase">Time to check in</span
      ><button
        class="size-6 cursor-pointer text-zinc-500"
        aria-label="Decide later"
        @click="deferred.add(task.occurrences.at(-1)!.id)"
      >
        ×
      </button>
    </header>
    <p class="m-0 leading-relaxed">{{ task.title }}</p>
    <p class="m-0 text-caption text-zinc-500">
      Due <WhenText :iso="new Date(task.occurrences.at(-1)!.dueMs!).toISOString()" />. If Rune was
      away, confirm what actually happened.
    </p>
    <div class="flex flex-wrap gap-2">
      <button
        :disabled="workspace.pending.value"
        class="cursor-pointer rounded-full bg-zinc-100 px-3 py-2 text-zinc-950 disabled:opacity-40"
        v-if="task.occurrences.at(-1)?.state === 'pending' || task.executionMode === 'analysis'"
        @click="
          workspace.action(
            task,
            task.executionMode === 'manual'
              ? 'start'
              : task.executionMode === 'analysis'
                ? 'analysis'
                : 'run',
          )
        "
      >
        {{
          task.executionMode === 'manual'
            ? 'Begin activity'
            : task.executionMode === 'analysis'
              ? 'Choose capture'
              : 'Run now'
        }}</button
      ><button
        :disabled="workspace.pending.value"
        class="cursor-pointer px-2 py-2 text-zinc-400"
        @click="workspace.action(task, 'done')"
      >
        Already done</button
      ><button
        :disabled="workspace.pending.value"
        class="cursor-pointer px-2 py-2 text-zinc-400"
        @click="workspace.action(task, 'skip')"
      >
        Skip</button
      ><button class="cursor-pointer px-2 py-2 text-zinc-400" @click="agent.openWorkspace('tasks')">
        Open tasks
      </button>
    </div>
    <p v-if="workspace.error.value" role="alert" class="m-0 text-[#ff6b72]">
      {{ workspace.error.value }}
    </p>
  </section>
</template>