<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, ref, watch } from 'vue'
import type { FlightRun } from '@/config/flight'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import { taskProgress } from '@/config/workspaceProgress'
import WorkProgress from './WorkProgress.vue'
import WhenText from './WhenText.vue'
import TaskPreview from './TaskPreview.vue'
const props = defineProps<{
  workspace: WorkspaceTasksModel
  history: FlightRun[]
  goals: IntakeEntry[]
  active: boolean
  parentFilter?: string | null
  focusTaskId?: string | null
}>()
const emit = defineEmits<{
  open: [id: string]
  goal: [id: string]
  schedule: [task: WorkspaceTask]
  organize: [run: FlightRun]
  clear: []
  remove: [id: string]
}>()
const meta = ref<Record<string, [number, number]>>({})
const error = ref('')
const notice = ref('')
const ownedRuns = computed(
  () =>
    new Set(
      props.workspace.tasks.value.flatMap((t) => [
        ...(t.legacyTaskId ? [t.legacyTaskId] : []),
        ...t.occurrences.flatMap((o) => (o.runId ? [o.runId] : [])),
      ]),
    ),
)
const tasks = computed(() =>
  props.workspace.tasks.value.filter(
    (t) => !props.parentFilter || t.parentGoalId === props.parentFilter,
  ),
)
const rows = computed(() =>
  [
    ...tasks.value.map((task) => ({
      id: task.id,
      time:
        task.occurrences.at(-1)?.completedMs ?? task.occurrences.at(-1)?.dueMs ?? task.createdMs,
      task,
      run: null as FlightRun | null,
    })),
    ...(!props.parentFilter
      ? props.history
          .filter((run) => !ownedRuns.value.has(run.id))
          .map((run) => ({
            id: run.id,
            time: run.startedUtc ? Date.parse(run.startedUtc) : 0,
            task: null as WorkspaceTask | null,
            run,
          }))
      : []),
  ].sort((a, b) => b.time - a.time),
)

watch(
  [() => props.active, () => props.history],
  async () => {
    if (!props.active) return
    try {
      meta.value =
        (await invoke<Record<string, [number, number]>>('workspace_history_progress', {
          ids: props.history.map((run) => run.id),
        })) ?? {}
    } catch (e) {
      error.value = String(e)
    }
  },
  { immediate: true },
)
watch([() => props.focusTaskId, () => props.active], async () => {
  if (!props.focusTaskId || !props.active) return
  await nextTick()
  const element = [...document.querySelectorAll<HTMLElement>('[data-work-task]')].find(
    (node) => node.dataset.workTask === props.focusTaskId,
  )
  element?.scrollIntoView?.({ block: 'nearest' })
})
function goalTitle(id: string) {
  return props.goals.find((g) => g.id === id)?.approvedGoal?.outcome ?? 'Parent goal unavailable'
}
function legacyProgress(run: FlightRun) {
  const p = meta.value[run.id]
  return run.outcome === 'succeeded' ? 100 : p?.[1] ? Math.round((p[0] / p[1]) * 100) : null
}
async function copy(run: FlightRun) {
  try {
    const text = await invoke<string | null>('flight_original_prompt', { id: run.id })
    if (!text) throw new Error('The original prompt was not retained')
    await navigator.clipboard.writeText(text)
    notice.value = 'Original prompt copied'
  } catch (e) {
    error.value = String(e)
  }
}
function previewRun(row: { task: WorkspaceTask | null; run: FlightRun | null }): FlightRun | null {
  if (row.run) return row.run
  const ids = [...(row.task?.occurrences ?? [])]
    .reverse()
    .flatMap((o) => (o.runId ? [o.runId] : []))
  if (row.task?.legacyTaskId) ids.push(row.task.legacyTaskId)
  return ids.map((id) => props.history.find((run) => run.id === id)).find(Boolean) ?? null
}
</script>
<template>
  <section aria-label="Task activity timeline" class="flex flex-col gap-5">
    <header class="flex flex-wrap items-center justify-between gap-3">
      <RuneButton variant="ghost" size="sm" v-if="parentFilter" @click="emit('clear')">
        Showing goal tasks · Clear filter ×
      </RuneButton>
    </header>
    <p v-if="error || workspace.error.value" role="alert" class="m-0 text-rune-caption text-error">
      {{ error || workspace.error.value }}
    </p>
    <p v-if="notice" role="status" class="m-0 text-rune-caption text-rune-text-2">{{ notice }}</p>
    <div class="flex flex-col gap-rune-1">
      <p v-if="!rows.length" class="m-0 text-rune-body text-rune-text-2">
        No tasks yet. Use + New task, or run a request from the Rune pill.
      </p>
      <article
        v-for="row in rows"
        :key="row.id"
        :data-work-task="row.id"
        class="relative flex min-w-0 flex-col gap-3 rounded-rune-tile bg-rune-surface p-rune-2"
        :class="focusTaskId === row.id ? 'bg-rune-raised' : ''"
      >
        <div class="flex flex-col gap-5 sm:flex-row">
          <TaskPreview
            :run="previewRun(row)"
            @open="previewRun(row) && emit('open', previewRun(row)!.id)"
          />
          <div class="flex min-w-0 flex-1 flex-col gap-3">
            <header class="flex flex-wrap items-start justify-between gap-3">
              <h3
                class="m-0 max-w-2xl line-clamp-2 text-rune-body text-rune-ink"
                :title="row.task?.title ?? row.run?.task"
              >
                {{ row.task?.title ?? row.run?.task }}
              </h3>
              <WhenText
                v-if="row.time"
                :iso="new Date(row.time).toISOString()"
                class="text-rune-heading text-rune-text-2"
              />
              <span v-else class="text-rune-heading text-rune-text-2">Date unavailable</span>
            </header>
            <template v-if="row.task"
              ><div class="flex flex-wrap items-center gap-3 text-rune-heading text-rune-text-2">
                <span class="capitalize">{{
                  row.task.paused ? 'Paused' : (row.task.occurrences.at(-1)?.state ?? 'Scheduled')
                }}</span
                ><span v-if="row.task.schedule"
                  >Next: <WhenText :iso="new Date(row.task.schedule.nextMs).toISOString()" /> ·
                  {{ row.task.schedule.repeat }} · {{ row.task.schedule.zone }}</span
                ><span v-if="row.task.occurrences.at(-1)?.reported"
                  >Completion reported by you</span
                >
              </div>
              <RuneButton
                variant="ghost"
                size="sm"
                v-if="row.task.parentGoalId"
                class="self-start"
                @click="emit('goal', row.task.parentGoalId)"
              >
                ← {{ goalTitle(row.task.parentGoalId) }}</RuneButton
              ><WorkProgress
                :value="taskProgress(row.task)"
                label="Latest occurrence progress"
                :detail="
                  row.task.executionMode === 'manual'
                    ? 'Manual activity; completion is reported by you.'
                    : row.task.totalSteps
                      ? `${row.task.completedSteps} of ${row.task.totalSteps} steps completed`
                      : row.task.occurrences.at(-1)?.state === 'completed'
                        ? 'Occurrence completed'
                        : 'Execution steps have not been measured yet.'
                "
              />
              <div class="flex flex-wrap items-center gap-2 text-rune-caption">
                <template
                  v-if="
                    ['pending', 'in_progress'].includes(row.task.occurrences.at(-1)?.state ?? '')
                  "
                  ><RuneButton
                    variant="primary"
                    :disabled="workspace.pending.value"
                    v-if="
                      row.task.occurrences.at(-1)?.state === 'pending' ||
                      row.task.executionMode === 'analysis'
                    "
                    @click="
                      workspace.action(
                        row.task,
                        row.task.executionMode === 'manual'
                          ? 'start'
                          : row.task.executionMode === 'analysis'
                            ? 'analysis'
                            : 'run',
                      )
                    "
                  >
                    {{
                      row.task.executionMode === 'manual'
                        ? 'Begin activity'
                        : row.task.executionMode === 'analysis'
                          ? 'Choose capture'
                          : 'Run now'
                    }}</RuneButton
                  ><RuneButton
                    variant="ghost"
                    size="sm"
                    :disabled="workspace.pending.value"
                    @click="workspace.action(row.task, 'done')"
                  >
                    Mark done</RuneButton
                  ><RuneButton
                    variant="ghost"
                    size="sm"
                    :disabled="workspace.pending.value"
                    @click="workspace.action(row.task, 'skip')"
                  >
                    Skip
                  </RuneButton></template
                ><RuneButton
                  variant="ghost"
                  size="sm"
                  v-if="
                    ['failed', 'interrupted', 'cancelled'].includes(
                      row.task.occurrences.at(-1)?.state ?? '',
                    )
                  "
                  :disabled="workspace.pending.value"
                  @click="workspace.action(row.task, 'retry')"
                >
                  Prepare retry</RuneButton
                ><RuneButton
                  variant="ghost"
                  size="sm"
                  v-if="
                    ['failed', 'interrupted', 'cancelled'].includes(
                      row.task.occurrences.at(-1)?.state ?? '',
                    )
                  "
                  :disabled="workspace.pending.value"
                  @click="workspace.action(row.task, 'skip')"
                >
                  Skip occurrence</RuneButton
                ><RuneButton
                  variant="ghost"
                  size="sm"
                  v-if="row.task.occurrences.at(-1)?.runId"
                  @click="emit('open', row.task.occurrences.at(-1)!.runId!)"
                >
                  Review execution</RuneButton
                ><RuneButton variant="ghost" size="sm" @click="emit('schedule', row.task)">
                  Schedule</RuneButton
                ><RuneButton
                  variant="ghost"
                  size="sm"
                  v-if="row.task.schedule"
                  :disabled="workspace.pending.value"
                  @click="workspace.action(row.task, row.task.paused ? 'resume' : 'pause')"
                >
                  {{ row.task.paused ? 'Resume reminders' : 'Pause reminders' }}
                </RuneButton>
              </div>
              <label class="flex max-w-md items-center gap-3 text-rune-heading text-rune-text-2"
                >Parent goal<select
                  :value="row.task.parentGoalId ?? ''"
                  :disabled="workspace.pending.value"
                  class="min-w-0 rounded-rune-tile bg-rune-surface p-2 text-rune-text-2"
                  @change="
                    workspace.link(row.task, ($event.target as HTMLSelectElement).value || null)
                  "
                >
                  <option value="">Standalone</option>
                  <option
                    v-for="goal in goals.filter((g) => g.class === row.task!.class)"
                    :key="goal.id"
                    :value="goal.id"
                  >
                    {{ goal.approvedGoal?.outcome ?? goal.text }}
                  </option>
                </select></label
              >
              <details v-if="row.task.instructions" class="text-rune-heading text-rune-text-2">
                <summary class="cursor-pointer">Task instructions</summary>
                <p class="whitespace-pre-wrap leading-relaxed text-rune-text-2">
                  {{ row.task.instructions }}
                </p>
              </details>
              <details
                v-if="row.task.occurrences.length > 1"
                class="text-rune-heading text-rune-text-2"
              >
                <summary class="cursor-pointer">
                  {{ row.task.occurrences.length }} occurrences and attempts
                </summary>
                <div
                  v-for="occurrence in [...row.task.occurrences].reverse()"
                  :key="occurrence.id"
                  class="flex justify-between gap-3 py-2"
                >
                  <span
                    ><WhenText
                      v-if="occurrence.dueMs"
                      :iso="new Date(occurrence.dueMs).toISOString()"
                    /><span v-else>Manual task</span> · {{ occurrence.state
                    }}{{ occurrence.reported ? ' · user reported' : '' }}</span
                  ><RuneButton
                    variant="ghost"
                    size="sm"
                    v-if="occurrence.runId"
                    @click="emit('open', occurrence.runId)"
                  >
                    Review →
                  </RuneButton>
                </div>
              </details>
              <p v-if="row.task.error" role="alert" class="m-0 text-rune-caption text-error">
                {{ row.task.error }}
              </p></template
            >
            <template v-else-if="row.run"
              ><p class="m-0 text-rune-heading text-rune-text-2">
                {{ row.run.outcome ?? 'In progress' }} · {{ row.run.source }}
              </p>
              <WorkProgress
                :value="legacyProgress(row.run)"
                label="Task progress"
                :detail="
                  meta[row.run.id]?.[1]
                    ? `${meta[row.run.id]![0]} of ${meta[row.run.id]![1]} steps completed`
                    : 'Older records may not contain a measurable step plan.'
                "
              />
              <div class="flex flex-wrap gap-3 text-rune-caption">
                <RuneButton variant="ghost" size="sm" @click="emit('open', row.run.id)">
                  Review →</RuneButton
                ><RuneButton
                  variant="ghost"
                  size="sm"
                  v-if="
                    ['', 'built-in', 'task-request', 'screen-insight', 'rune'].includes(
                      row.run.source,
                    )
                  "
                  @click="emit('organize', row.run)"
                  :disabled="row.run.active"
                >
                  Link goal or schedule</RuneButton
                ><RuneButton variant="ghost" size="sm" @click="copy(row.run)">
                  Copy prompt</RuneButton
                ><RuneButton
                  variant="ghost"
                  size="sm"
                  :disabled="row.run.active"
                  @click="emit('remove', row.run.id)"
                >
                  Delete
                </RuneButton>
              </div></template
            >
          </div>
        </div>
      </article>
    </div>
  </section>
</template>