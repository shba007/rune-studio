<script setup lang="ts">
import RuneIconButton from './RuneIconButton.vue'
import RuneButton from './RuneButton.vue'
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
    class="flex flex-col gap-rune-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-body text-rune-ink"
  >
    <header class="flex items-center justify-between">
      <span class="text-rune-caption text-rune-text-2 uppercase">Time to check in</span
      ><RuneIconButton
        variant="ghost"
        size="xs"
        label="Decide later"
        @click="deferred.add(task.occurrences.at(-1)!.id)"
      >
        ×
      </RuneIconButton>
    </header>
    <p class="m-0 leading-relaxed">{{ task.title }}</p>
    <p class="m-0 text-rune-caption text-rune-text-2">
      Due <WhenText :iso="new Date(task.occurrences.at(-1)!.dueMs!).toISOString()" />. If Rune was
      away, confirm what actually happened.
    </p>
    <div class="flex flex-wrap gap-2">
      <RuneButton
        variant="primary"
        :disabled="workspace.pending.value"
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
        }}</RuneButton
      ><RuneButton
        variant="ghost"
        size="sm"
        :disabled="workspace.pending.value"
        @click="workspace.action(task, 'done')"
      >
        Already done</RuneButton
      ><RuneButton
        variant="ghost"
        size="sm"
        :disabled="workspace.pending.value"
        @click="workspace.action(task, 'skip')"
      >
        Skip</RuneButton
      ><RuneButton variant="ghost" size="sm" @click="agent.openWorkspace('tasks')">
        Open tasks
      </RuneButton>
    </div>
    <p v-if="workspace.error.value" role="alert" class="m-0 text-error">
      {{ workspace.error.value }}
    </p>
  </section>
</template>