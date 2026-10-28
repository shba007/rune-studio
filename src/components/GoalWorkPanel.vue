<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import { planProgress, outcomeProgress } from '@/config/workspaceProgress'
import WorkProgress from './WorkProgress.vue'
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
  <section
    aria-label="Goal progress and tasks"
    class="flex flex-col gap-rune-3 border-t border-white/10 pt-rune-3"
  >
    <div class="grid gap-6 sm:grid-cols-2">
      <WorkProgress
        :value="plan.value"
        label="Plan progress · due work"
        :detail="`${plan.done} of ${plan.total} created or due occurrences completed. Recurring work can continue; future occurrences are not counted yet.`"
      /><WorkProgress
        :value="outcomeProgress(entry.outcomeMetric)"
        label="Outcome progress · user reported"
        :detail="
          entry.outcomeMetric
            ? `${entry.outcomeMetric.current} / ${entry.outcomeMetric.target} ${entry.outcomeMetric.unit} · starting at ${entry.outcomeMetric.baseline}. ${entry.outcomeMetric.note}`
            : 'Set a numeric measure to track the real outcome separately from task completion.'
        "
      />
    </div>
    <div class="flex flex-wrap gap-rune-2 text-rune-body">
      <button
        class="cursor-pointer text-zinc-200 hover:text-white"
        @click="emit('tasks', entry.id)"
      >
        {{ children.length }} linked task{{ children.length === 1 ? '' : 's' }} →</button
      ><button
        class="cursor-pointer text-zinc-400 hover:text-white"
        @click="emit('create', entry.id)"
      >
        + Add task</button
      ><button
        :disabled="entry.status !== 'goal_ready'"
        class="cursor-pointer text-zinc-400 hover:text-white disabled:opacity-40"
        @click="edit"
      >
        {{ entry.outcomeMetric ? 'Record outcome progress' : 'Set outcome measure' }}
      </button>
    </div>
    <ul v-if="children.length" class="m-0 flex list-none flex-col gap-2 p-0 text-rune-body">
      <li v-for="task in children.slice(0, 3)" :key="task.id">
        <button
          class="flex w-full cursor-pointer items-center justify-between gap-rune-2 text-left text-zinc-400 hover:text-white"
          @click="emit('task', task.id)"
        >
          <span>{{ task.title }}</span
          ><span class="shrink-0 text-rune-caption"
            >{{ task.occurrences.at(-1)?.state ?? 'scheduled' }} →</span
          >
        </button>
      </li>
    </ul>
    <form
      v-if="editing"
      class="grid gap-rune-2 rounded-xl bg-white/[0.025] p-4 sm:grid-cols-3"
      aria-label="Record goal outcome"
      @submit.prevent="save"
    >
      <label class="flex flex-col gap-2 text-rune-body text-zinc-400"
        >Starting value<input
          v-model="baseline"
          type="number"
          step="any"
          required
          :disabled="!!entry.outcomeMetric || pending"
          class="min-w-0 rounded-lg bg-white/5 p-2 text-zinc-100" /></label
      ><label class="flex flex-col gap-2 text-rune-body text-zinc-400"
        >Current value<input
          v-model="current"
          type="number"
          step="any"
          required
          :disabled="pending"
          class="min-w-0 rounded-lg bg-white/5 p-2 text-zinc-100" /></label
      ><label class="flex flex-col gap-2 text-rune-body text-zinc-400"
        >Target value<input
          v-model="target"
          type="number"
          step="any"
          required
          :disabled="!!entry.outcomeMetric || pending"
          class="min-w-0 rounded-lg bg-white/5 p-2 text-zinc-100" /></label
      ><label class="flex flex-col gap-2 text-rune-body text-zinc-400"
        >Unit<input
          v-model="unit"
          required
          maxlength="80"
          :disabled="!!entry.outcomeMetric || pending"
          placeholder="For example, km or currency"
          class="rounded-lg bg-white/5 p-2 text-zinc-100" /></label
      ><label class="flex flex-col gap-2 text-rune-body text-zinc-400 sm:col-span-2"
        >Progress note<input
          v-model="note"
          maxlength="2000"
          :disabled="pending"
          class="rounded-lg bg-white/5 p-2 text-zinc-100"
      /></label>
      <p class="m-0 text-rune-caption text-zinc-500 sm:col-span-3">
        This is your reported outcome, not automatically verified evidence. Progress is measured
        from the starting value toward the target; lower targets are supported.
      </p>
      <button
        :disabled="pending"
        class="min-h-9 cursor-pointer rounded-full bg-zinc-100 px-4 py-2 text-rune-body text-zinc-950"
      >
        Save measurement
      </button>
      <p v-if="error" role="alert" class="m-0 text-rune-body text-error sm:col-span-3">
        {{ error }}
      </p>
    </form>
  </section>
</template>