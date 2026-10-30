<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import { planProgress, outcomeProgress } from '@/config/workspaceProgress'
import WorkProgress from './WorkProgress.vue'
import WorkRow from './WorkRow.vue'
const props = defineProps<{ entry: IntakeEntry; workspace: WorkspaceTasksModel }>()
const emit = defineEmits<{
  refresh: []
  tasks: [goalId: string]
  task: [id: string]
  create: [goalId: string]
}>()
const children = computed(() =>
  props.workspace.tasks.value.filter((t) => t.parentGoalId === props.entry.id),
)
const plan = computed(() => planProgress(children.value))
const editing = ref(false)
const baseline = ref('')
const current = ref('')
const target = ref('')
const unit = ref('')
const note = ref('')
const pending = ref(false)
const error = ref('')
const field =
  'h-10 min-w-0 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3 disabled:opacity-50'
function edit() {
  const m = props.entry.outcomeMetric
  baseline.value = m ? String(m.baseline) : ''
  current.value = m ? String(m.current) : ''
  target.value = m ? String(m.target) : ''
  unit.value = m?.unit ?? ''
  note.value = ''
  editing.value = !editing.value
}
async function save() {
  if (pending.value) return
  pending.value = true
  error.value = ''
  try {
    if (
      [baseline.value, current.value, target.value].some(
        (v) => !v.trim() || !Number.isFinite(Number(v)),
      )
    )
      throw new Error('Enter finite starting, current, and target values')
    await invoke('goal_measurement', {
      id: props.entry.id,
      revision: props.entry.revision,
      baseline: Number(baseline.value),
      current: Number(current.value),
      target: Number(target.value),
      unit: unit.value,
      note: note.value,
    })
    editing.value = false
    emit('refresh')
  } catch (e) {
    error.value = String(e)
  } finally {
    pending.value = false
  }
}
</script>
<template>
  <section aria-label="Goal progress and tasks" class="@container flex flex-col gap-rune-2">
    <div class="grid gap-0.5 @md:grid-cols-2">
      <div
        class="bg-rune-surface px-rune-2 py-rune-2 max-[28rem]:first:rounded-t-rune-tile @md:rounded-l-rune-tile"
      >
        <WorkProgress
          :value="plan.value"
          label="Plan progress · due work"
          :detail="`${plan.done} of ${plan.total} created or due occurrences completed. Recurring work can continue; future occurrences are not counted yet.`"
        />
      </div>
      <div
        class="bg-rune-surface px-rune-2 py-rune-2 max-[28rem]:last:rounded-b-rune-tile @md:rounded-r-rune-tile"
      >
        <WorkProgress
          :value="outcomeProgress(entry.outcomeMetric)"
          label="Outcome progress · user reported"
          :detail="
            entry.outcomeMetric
              ? `${entry.outcomeMetric.current} / ${entry.outcomeMetric.target} ${entry.outcomeMetric.unit} · starting at ${entry.outcomeMetric.baseline}. ${entry.outcomeMetric.note}`
              : 'Set a numeric measure to track the real outcome separately from task completion.'
          "
        />
      </div>
    </div>
    <div class="flex flex-wrap items-center gap-rune-1">
      <RuneButton variant="secondary" size="sm" @click="emit('create', entry.id)">
        + Add task
      </RuneButton>
      <RuneButton
        variant="secondary"
        size="sm"
        :disabled="entry.status !== 'goal_ready'"
        @click="edit"
      >
        {{ entry.outcomeMetric ? 'Record outcome progress' : 'Set outcome measure' }}
      </RuneButton>
      <RuneButton variant="ghost" size="sm" @click="emit('tasks', entry.id)">
        {{ children.length }} linked task{{ children.length === 1 ? '' : 's' }} →
      </RuneButton>
    </div>
    <div v-if="children.length" class="flex flex-col gap-0.5">
      <WorkRow
        v-for="task in children.slice(0, 3)"
        :key="task.id"
        :title="task.title"
        :meta="task.occurrences.at(-1)?.state ?? 'scheduled'"
        interactive
        @open="emit('task', task.id)"
      />
    </div>
    <form
      v-if="editing"
      class="grid gap-rune-2 rounded-rune-tile bg-rune-surface p-rune-2 sm:grid-cols-3"
      aria-label="Record goal outcome"
      @submit.prevent="save"
    >
      <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
        >Starting value<input
          v-model="baseline"
          type="number"
          step="any"
          required
          :disabled="!!entry.outcomeMetric || pending"
          :class="field"
      /></label>
      <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
        >Current value<input
          v-model="current"
          type="number"
          step="any"
          required
          :disabled="pending"
          :class="field"
      /></label>
      <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
        >Target value<input
          v-model="target"
          type="number"
          step="any"
          required
          :disabled="!!entry.outcomeMetric || pending"
          :class="field"
      /></label>
      <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
        >Unit<input
          v-model="unit"
          required
          maxlength="80"
          :disabled="!!entry.outcomeMetric || pending"
          placeholder="For example, km or currency"
          :class="field"
      /></label>
      <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3 sm:col-span-2"
        >Progress note<input v-model="note" maxlength="2000" :disabled="pending" :class="field"
      /></label>
      <p class="m-0 text-rune-caption text-rune-text-3 sm:col-span-3">
        This is your reported outcome, not automatically verified evidence. Progress is measured
        from the starting value toward the target; lower targets are supported.
      </p>
      <RuneButton variant="primary" size="sm" class="self-start" :disabled="pending">
        Save measurement
      </RuneButton>
      <p v-if="error" role="alert" class="m-0 text-rune-body text-error sm:col-span-3">
        {{ error }}
      </p>
    </form>
  </section>
</template>