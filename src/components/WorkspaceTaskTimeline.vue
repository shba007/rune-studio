<script setup lang="ts">
import AppIcon from './AppIcon.vue'
import HubGroup from './HubGroup.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
import RuneIconButton from './RuneIconButton.vue'
import RuneMenu from './RuneMenu.vue'
import RuneNotice from './RuneNotice.vue'
import WorkEmpty from './WorkEmpty.vue'
import WorkRow from './WorkRow.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, nextTick, ref, watch } from 'vue'
import type { FlightRun } from '@/config/flight'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import { taskProgress } from '@/config/workspaceProgress'
import WorkProgress from './WorkProgress.vue'
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

type Row = { id: string; time: number; task: WorkspaceTask | null; run: FlightRun | null }
type Tone = 'idle' | 'ok' | 'warn' | 'down' | 'busy'
type Bucket = 'active' | 'scheduled' | 'done' | 'failed'

const FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'active', label: 'Active' },
  { value: 'scheduled', label: 'Scheduled' },
  { value: 'done', label: 'Done' },
  { value: 'failed', label: 'Failed' },
]
const filter = ref('all')
const expanded = ref(new Set<string>())
function toggle(id: string) {
  const next = new Set(expanded.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  expanded.value = next
}

const clock = (ms: number) =>
  new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
const day = (ms: number) =>
  new Date(ms).toLocaleDateString([], { weekday: 'short', day: 'numeric', month: 'short' })
const FAILED = ['failed', 'interrupted', 'cancelled']

function info(row: Row): { tone: Tone; bucket: Bucket; meta: string } {
  const time = row.time ? clock(row.time) : 'Date unavailable'
  if (row.task) {
    const occurrence = row.task.occurrences.at(-1)
    const state = row.task.paused ? 'paused' : (occurrence?.state ?? 'scheduled')
    const tone: Tone =
      state === 'completed'
        ? 'ok'
        : state === 'in_progress'
          ? 'busy'
          : FAILED.includes(state)
            ? 'down'
            : state === 'paused' || state === 'skipped'
              ? 'warn'
              : 'idle'
    const bucket: Bucket =
      state === 'completed'
        ? 'done'
        : state === 'in_progress'
          ? 'active'
          : FAILED.includes(state)
            ? 'failed'
            : 'scheduled'
    const parts = [state.replace('_', ' ').replace(/^./, (c) => c.toUpperCase()), time]
    if (row.task.schedule)
      parts.push(`Next ${day(row.task.schedule.nextMs)} · ${row.task.schedule.repeat}`)
    if (occurrence?.reported) parts.push('Completion reported by you')
    return { tone, bucket, meta: parts.join(' · ') }
  }
  const run = row.run!
  const tone: Tone = run.active
    ? 'busy'
    : run.outcome === 'succeeded'
      ? 'ok'
      : run.outcome
        ? 'down'
        : 'idle'
  const bucket: Bucket = run.active
    ? 'active'
    : run.outcome === 'succeeded'
      ? 'done'
      : run.outcome
        ? 'failed'
        : 'active'
  const label = run.outcome ?? 'In progress'
  return {
    tone,
    bucket,
    meta: [label.replace(/^./, (c) => c.toUpperCase()), time, run.source]
      .filter(Boolean)
      .join(' · '),
  }
}

const visibleRows = computed(() =>
  rows.value.filter((row) => filter.value === 'all' || info(row).bucket === filter.value),
)
const groups = computed(() => {
  const today = new Date().toDateString()
  const yesterday = new Date(Date.now() - 86_400_000).toDateString()
  const out: { key: string; title: string; rows: Row[] }[] = []
  for (const row of visibleRows.value) {
    const key = row.time ? new Date(row.time).toDateString() : 'unknown'
    let group = out.find((item) => item.key === key)
    if (!group) {
      group = {
        key,
        title:
          key === 'unknown'
            ? 'Date unavailable'
            : key === today
              ? 'Today'
              : key === yesterday
                ? 'Yesterday'
                : day(row.time),
        rows: [],
      }
      out.push(group)
    }
    group.rows.push(row)
  }
  return out
})

function primary(row: Row): { label: string; run: () => void } | null {
  const task = row.task
  if (!task) return null
  const state = task.occurrences.at(-1)?.state ?? ''
  if (!['pending', 'in_progress'].includes(state)) return null
  if (state !== 'pending' && task.executionMode !== 'analysis') return null
  const action =
    task.executionMode === 'manual'
      ? 'start'
      : task.executionMode === 'analysis'
        ? 'analysis'
        : 'run'
  const label =
    task.executionMode === 'manual'
      ? 'Begin activity'
      : task.executionMode === 'analysis'
        ? 'Choose capture'
        : 'Run now'
  return { label, run: () => void props.workspace.action(task, action) }
}

function menu(row: Row) {
  const items: { id: string; label: string; disabled?: boolean; danger?: boolean }[] = []
  const busy = props.workspace.pending.value
  if (row.task) {
    const task = row.task
    const occurrence = task.occurrences.at(-1)
    const state = occurrence?.state ?? ''
    if (occurrence?.runId) items.push({ id: 'review', label: 'Review execution' })
    if (['pending', 'in_progress'].includes(state)) {
      items.push({ id: 'done', label: 'Mark done', disabled: busy })
      items.push({ id: 'skip', label: 'Skip', disabled: busy })
    }
    if (FAILED.includes(state)) {
      items.push({ id: 'retry', label: 'Prepare retry', disabled: busy })
      items.push({ id: 'skip', label: 'Skip occurrence', disabled: busy })
    }
    items.push({ id: 'schedule', label: 'Schedule' })
    if (task.schedule)
      items.push({
        id: task.paused ? 'resume' : 'pause',
        label: task.paused ? 'Resume reminders' : 'Pause reminders',
        disabled: busy,
      })
  } else if (row.run) {
    items.push({ id: 'review', label: 'Review' })
    if (['', 'built-in', 'task-request', 'screen-insight', 'rune'].includes(row.run.source))
      items.push({ id: 'organize', label: 'Link goal or schedule', disabled: !!row.run.active })
    items.push({ id: 'copy', label: 'Copy prompt' })
    items.push({ id: 'delete', label: 'Delete', disabled: !!row.run.active, danger: true })
  }
  return items
}

function act(row: Row, id: string) {
  const task = row.task
  const run = row.run
  if (id === 'review') emit('open', task ? task.occurrences.at(-1)!.runId! : run!.id)
  else if (id === 'schedule' && task) emit('schedule', task)
  else if (id === 'organize' && run) emit('organize', run)
  else if (id === 'copy' && run) void copy(run)
  else if (id === 'delete' && run) emit('remove', run.id)
  else if (task && ['done', 'skip', 'retry', 'pause', 'resume'].includes(id))
    void props.workspace.action(task, id as 'done' | 'skip' | 'retry' | 'pause' | 'resume')
}

function progressDetail(row: Row) {
  if (row.task) {
    const task = row.task
    return task.executionMode === 'manual'
      ? 'Manual activity; completion is reported by you.'
      : task.totalSteps
        ? `${task.completedSteps} of ${task.totalSteps} steps completed`
        : task.occurrences.at(-1)?.state === 'completed'
          ? 'Occurrence completed'
          : 'Execution steps have not been measured yet.'
  }
  const run = row.run!
  return meta.value[run.id]?.[1]
    ? `${meta.value[run.id]![0]} of ${meta.value[run.id]![1]} steps completed`
    : 'Older records may not contain a measurable step plan.'
}

function goalOptions(task: WorkspaceTask) {
  return [
    { value: '', label: 'Standalone' },
    ...props.goals
      .filter((goal) => goal.class === task.class)
      .map((goal) => ({ value: goal.id, label: goal.approvedGoal?.outcome ?? goal.text })),
  ]
}
</script>

<template>
  <section aria-label="Task activity timeline" class="flex flex-col gap-rune-3">
    <div class="flex flex-wrap items-center justify-between gap-rune-2">
      <RuneChoice v-model="filter" label="Filter tasks" segments :options="FILTERS" />
      <div class="flex items-center gap-rune-1">
        <RuneButton v-if="parentFilter" variant="ghost" size="sm" @click="emit('clear')">
          Showing goal tasks · Clear filter ×
        </RuneButton>
        <slot name="toolbar" />
      </div>
    </div>
    <RuneNotice v-if="error || workspace.error.value" tone="error">
      {{ error || workspace.error.value }}
    </RuneNotice>
    <RuneNotice v-if="notice">{{ notice }}</RuneNotice>
    <WorkEmpty
      v-if="!rows.length"
      title="No tasks yet"
      hint="Use + New task, or run a request from the Rune pill."
    />
    <WorkEmpty v-else-if="!visibleRows.length" title="Nothing matches this filter" />
    <HubGroup v-for="group in groups" :key="group.key" :title="group.title">
      <WorkRow
        v-for="row in group.rows"
        :key="row.id"
        :data-work-task="row.id"
        :title="row.task?.title ?? row.run?.task ?? ''"
        :meta="info(row).meta"
        :state="info(row).tone"
        :selected="focusTaskId === row.id"
      >
        <template #leading>
          <TaskPreview
            :run="previewRun(row)"
            @open="previewRun(row) && emit('open', previewRun(row)!.id)"
          />
        </template>
        <template #actions>
          <RuneButton
            v-if="primary(row)"
            variant="primary"
            size="sm"
            :disabled="workspace.pending.value"
            @click="primary(row)!.run()"
          >
            {{ primary(row)!.label }}
          </RuneButton>
          <RuneMenu
            :label="`More actions: ${row.task?.title ?? row.run?.task}`"
            :items="menu(row)"
            @select="(id) => act(row, id)"
          />
          <RuneIconButton
            v-if="row.task"
            variant="ghost"
            size="sm"
            :label="expanded.has(row.id) ? 'Hide details' : 'Show details'"
            :aria-expanded="expanded.has(row.id)"
            @click="toggle(row.id)"
          >
            <AppIcon
              name="chevron"
              class="transition-transform"
              :class="expanded.has(row.id) && 'rotate-180'"
            />
          </RuneIconButton>
        </template>
        <WorkProgress
          :value="row.task ? taskProgress(row.task) : legacyProgress(row.run!)"
          :label="row.task ? 'Latest occurrence progress' : 'Task progress'"
          :detail="progressDetail(row)"
        />
        <div
          v-if="row.task && expanded.has(row.id)"
          class="flex flex-col gap-rune-2 rounded-rune-tile bg-rune-sunken p-rune-2"
        >
          <template v-if="row.task">
            <div class="flex flex-wrap items-center justify-between gap-rune-2">
              <span class="text-rune-body text-rune-ink">Parent goal</span>
              <div class="flex items-center gap-rune-1">
                <RuneButton
                  v-if="row.task.parentGoalId"
                  variant="ghost"
                  size="sm"
                  @click="emit('goal', row.task.parentGoalId)"
                >
                  ← {{ goalTitle(row.task.parentGoalId) }}
                </RuneButton>
                <RuneChoice
                  :model-value="row.task.parentGoalId ?? ''"
                  label="Parent goal"
                  menu
                  :disabled="workspace.pending.value"
                  :options="goalOptions(row.task)"
                  @update:model-value="(value) => workspace.link(row.task!, value || null)"
                />
              </div>
            </div>
            <p v-if="row.task.instructions" class="m-0 text-rune-caption text-rune-text-2">
              <span class="block text-rune-text-3">Task instructions</span>
              <span class="whitespace-pre-wrap">{{ row.task.instructions }}</span>
            </p>
            <div v-if="row.task.occurrences.length > 1" class="flex flex-col gap-1">
              <span class="text-rune-caption text-rune-text-3">
                {{ row.task.occurrences.length }} occurrences and attempts
              </span>
              <div
                v-for="occurrence in [...row.task.occurrences].reverse()"
                :key="occurrence.id"
                class="flex items-center justify-between gap-3 text-rune-caption text-rune-text-2"
              >
                <span
                  >{{ occurrence.dueMs ? day(occurrence.dueMs) : 'Manual task' }} ·
                  {{ occurrence.state }}{{ occurrence.reported ? ' · user reported' : '' }}</span
                >
                <RuneButton
                  v-if="occurrence.runId"
                  variant="ghost"
                  size="sm"
                  @click="emit('open', occurrence.runId)"
                >
                  Review →
                </RuneButton>
              </div>
            </div>
            <RuneNotice v-if="row.task.error" tone="error">{{ row.task.error }}</RuneNotice>
          </template>
        </div>
      </WorkRow>
    </HubGroup>
  </section>
</template>