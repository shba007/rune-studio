<script setup lang="ts">
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
      <button
        v-if="parentFilter"
        class="cursor-pointer text-label text-zinc-400"
        @click="emit('clear')"
      >
        Showing goal tasks · Clear filter ×
      </button>
    </header>
    <p v-if="error || workspace.error.value" role="alert" class="m-0 text-label text-[#ff6b72]">
      {{ error || workspace.error.value }}
    </p>
    <p v-if="notice" role="status" class="m-0 text-label text-zinc-400">{{ notice }}</p>
    <div class="ml-2 flex flex-col border-l border-white/10 pl-6">
      <p v-if="!rows.length" class="text-label text-zinc-500">
        No tasks here yet. Create a task or run a request to begin.
      </p>
      <article
        v-for="row in rows"
        :key="row.id"
        :data-work-task="row.id"
        class="relative flex min-w-0 flex-col gap-3 border-b border-white/8 py-5 last:border-0"
        :class="
          focusTaskId === row.id ? 'rounded-xl bg-white/[0.025] px-4 ring-1 ring-white/15' : ''
        "
      >
        <span
          class="absolute top-7 -left-[29px] size-2 rounded-full border border-zinc-500 bg-black"
          aria-hidden="true"
        />
        <div class="flex flex-col gap-5 sm:flex-row">
          <TaskPreview
            :run="previewRun(row)"
            @open="previewRun(row) && emit('open', previewRun(row)!.id)"
          />
          <div class="flex min-w-0 flex-1 flex-col gap-3">
            <header class="flex flex-wrap items-start justify-between gap-3">
              <h3
                class="m-0 max-w-2xl line-clamp-2 text-body text-zinc-200"
                :title="row.task?.title ?? row.run?.task"
              >
                {{ row.task?.title ?? row.run?.task }}
              </h3>
              <WhenText
                v-if="row.time"
                :iso="new Date(row.time).toISOString()"
                class="text-caption text-zinc-500"
              />
              <span v-else class="text-caption text-zinc-500">Date unavailable</span>
            </header>
            <template v-if="row.task"
              ><div class="flex flex-wrap items-center gap-3 text-caption text-zinc-500">
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
              <button
                v-if="row.task.parentGoalId"
                class="self-start cursor-pointer text-label text-zinc-300 hover:text-white"
                @click="emit('goal', row.task.parentGoalId)"
              >
                ← {{ goalTitle(row.task.parentGoalId) }}</button
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
              <div class="flex flex-wrap items-center gap-2 text-label">
                <template
                  v-if="
                    ['pending', 'in_progress'].includes(row.task.occurrences.at(-1)?.state ?? '')
                  "
                  ><button
                    :disabled="workspace.pending.value"
                    class="cursor-pointer rounded-full bg-zinc-100 px-4 py-2 text-zinc-950 disabled:opacity-40"
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
                    }}</button
                  ><button
                    :disabled="workspace.pending.value"
                    class="cursor-pointer px-2 py-2 text-zinc-400"
                    @click="workspace.action(row.task, 'done')"
                  >
                    Mark done</button
                  ><button
                    :disabled="workspace.pending.value"
                    class="cursor-pointer px-2 py-2 text-zinc-500"
                    @click="workspace.action(row.task, 'skip')"
                  >
                    Skip
                  </button></template
                ><button
                  v-if="
                    ['failed', 'interrupted', 'cancelled'].includes(
                      row.task.occurrences.at(-1)?.state ?? '',
                    )
                  "
                  :disabled="workspace.pending.value"
                  class="cursor-pointer px-2 py-2 text-zinc-300"
                  @click="workspace.action(row.task, 'retry')"
                >
                  Prepare retry</button
                ><button
                  v-if="
                    ['failed', 'interrupted', 'cancelled'].includes(
                      row.task.occurrences.at(-1)?.state ?? '',
                    )
                  "
                  :disabled="workspace.pending.value"
                  class="cursor-pointer px-2 py-2 text-zinc-500"
                  @click="workspace.action(row.task, 'skip')"
                >
                  Skip occurrence</button
                ><button
                  v-if="row.task.occurrences.at(-1)?.runId"
                  class="cursor-pointer px-2 py-2 text-zinc-400"
                  @click="emit('open', row.task.occurrences.at(-1)!.runId!)"
                >
                  Review execution</button
                ><button
                  class="cursor-pointer px-2 py-2 text-zinc-400"
                  @click="emit('schedule', row.task)"
                >
                  Schedule</button
                ><button
                  v-if="row.task.schedule"
                  :disabled="workspace.pending.value"
                  class="cursor-pointer px-2 py-2 text-zinc-500"
                  @click="workspace.action(row.task, row.task.paused ? 'resume' : 'pause')"
                >
                  {{ row.task.paused ? 'Resume reminders' : 'Pause reminders' }}
                </button>
              </div>
              <label class="flex max-w-md items-center gap-3 text-caption text-zinc-500"
                >Parent goal<select
                  :value="row.task.parentGoalId ?? ''"
                  :disabled="workspace.pending.value"
                  class="min-w-0 rounded-lg bg-[#141414] p-2 text-zinc-300"
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
              <details v-if="row.task.instructions" class="text-caption text-zinc-500">
                <summary class="cursor-pointer">Task instructions</summary>
                <p class="whitespace-pre-wrap leading-relaxed text-zinc-300">
                  {{ row.task.instructions }}
                </p>
              </details>
              <details v-if="row.task.occurrences.length > 1" class="text-caption text-zinc-500">
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
                  ><button
                    v-if="occurrence.runId"
                    class="cursor-pointer text-zinc-300"
                    @click="emit('open', occurrence.runId)"
                  >
                    Review →
                  </button>
                </div>
              </details>
              <p v-if="row.task.error" role="alert" class="m-0 text-label text-[#ff6b72]">
                {{ row.task.error }}
              </p></template
            >
            <template v-else-if="row.run"
              ><p class="m-0 text-caption text-zinc-500">
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
              <div class="flex flex-wrap gap-3 text-label">
                <button class="cursor-pointer text-zinc-200" @click="emit('open', row.run.id)">
                  Review →</button
                ><button
                  v-if="
                    ['', 'built-in', 'task-request', 'screen-insight', 'rune'].includes(
                      row.run.source,
                    )
                  "
                  class="cursor-pointer text-zinc-400"
                  @click="emit('organize', row.run)"
                  :disabled="row.run.active"
                >
                  Link goal or schedule</button
                ><button class="cursor-pointer text-zinc-500" @click="copy(row.run)">
                  Copy prompt</button
                ><button
                  :disabled="row.run.active"
                  class="cursor-pointer text-zinc-500 disabled:opacity-30"
                  @click="emit('remove', row.run.id)"
                >
                  Delete
                </button>
              </div></template
            >
          </div>
        </div>
      </article>
    </div>
  </section>
</template>