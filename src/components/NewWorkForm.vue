<script setup lang="ts">
import RuneIconButton from './RuneIconButton.vue'
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
const props = defineProps<{
  kind: 'goal' | 'task'
  goals: IntakeEntry[]
  parentGoalId?: string | null
  task?: WorkspaceTask
  legacyRunId?: string
  initialTitle?: string
}>()
const emit = defineEmits<{ close: []; saved: [kind: 'goal' | 'task', id?: string] }>()
const title = ref(props.task?.title ?? props.initialTitle ?? '')
const executionMode = ref<'manual' | 'agent' | 'analysis'>('manual')
const parent = ref(props.parentGoalId ?? null)
const privacy = ref<IntakeEntry['class']>('general')
const scheduled = ref(!!props.task?.schedule)
const local = ref(props.task?.schedule?.nextLocal ?? '')
const repeat = ref(props.task?.schedule?.repeat ?? 'once')
const monthlyDay = ref<number | null>(
  props.task?.schedule?.anchorDay ??
    (props.task?.schedule ? Number(props.task.schedule.local.slice(8, 10)) : null),
)
watch(local, (value) => {
  if (!monthlyDay.value && value) monthlyDay.value = Number(value.slice(8, 10))
})
const zone = ref(props.task?.schedule?.zone ?? '')
const pending = ref(false)
const error = ref('')
const parentGoal = computed(() => props.goals.find((g) => g.id === parent.value))
onMounted(async () => {
  if (props.kind === 'task' && !zone.value)
    try {
      zone.value = await invoke<string>('workspace_task_zone')
    } catch (e) {
      error.value = String(e)
    }
})
async function submit() {
  if (pending.value) return
  pending.value = true
  error.value = ''
  try {
    let id: string | undefined
    const schedule = scheduled.value
      ? {
          local: local.value,
          zone: zone.value,
          repeat: repeat.value,
          anchorDay: repeat.value === 'monthly' ? monthlyDay.value : null,
        }
      : null
    if (props.kind === 'goal')
      id = await invoke<string>('workspace_new_goal', { text: title.value })
    else if (props.task) {
      await invoke('workspace_task_schedule', {
        id: props.task.id,
        revision: props.task.revision,
        schedule,
      })
      id = props.task.id
    } else if (props.legacyRunId)
      id = await invoke<string>('workspace_task_import', {
        runId: props.legacyRunId,
        parentGoalId: parent.value,
        class: parentGoal.value?.class ?? privacy.value,
        schedule,
      })
    else
      id = await invoke<string>('workspace_task_create', {
        title: title.value,
        executionMode: executionMode.value,
        parentGoalId: parent.value,
        class: parentGoal.value?.class ?? privacy.value,
        schedule,
      })
    emit('saved', props.kind, id)
  } catch (e) {
    error.value = String(e)
  } finally {
    pending.value = false
  }
}
</script>
<template>
  <form
    aria-label="Create or schedule work"
    class="mb-8 flex max-w-3xl flex-col gap-4 py-6 text-rune-caption"
    @submit.prevent="submit"
  >
    <header class="flex items-center justify-between">
      <h2 class="m-0 text-rune-body text-rune-ink">
        {{
          task
            ? 'Schedule task'
            : kind === 'goal'
              ? 'New goal'
              : legacyRunId
                ? 'Organize this task'
                : 'New task'
        }}
      </h2>
      <RuneIconButton
        variant="ghost"
        size="sm"
        :disabled="pending"
        label="Close creation form"
        @click="emit('close')"
      >
        ×
      </RuneIconButton>
    </header>
    <p v-if="task" class="m-0 text-rune-caption text-rune-text-2">{{ task.title }}</p>
    <fieldset :disabled="pending" class="m-0 flex flex-col gap-4 p-0">
      <label v-if="!task" class="flex flex-col gap-2 text-rune-text-2"
        >{{ kind === 'goal' ? 'What would you like to achieve?' : 'What needs to be done?'
        }}<textarea
          v-model="title"
          rows="2"
          maxlength="4000"
          required
          :readonly="!!legacyRunId"
          class="resize-y rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
        /></label
      ><template v-if="kind === 'task'">
        <label v-if="!task && !legacyRunId" class="flex flex-col gap-2 text-rune-text-2"
          >Who will do this task?<select
            v-model="executionMode"
            class="rounded-rune-tile bg-rune-raised p-3 text-rune-ink"
          >
            <option value="manual">I will do it · track my activity</option>
            <option value="agent">Rune · execute with computer/tools</option>
            <option value="analysis">Visual Analysis · choose a fresh capture</option>
          </select></label
        >
        <template v-if="!task"
          ><label class="flex flex-col gap-2 text-rune-text-2"
            >Parent goal<select
              v-model="parent"
              class="rounded-rune-tile bg-rune-raised p-3 text-rune-ink"
            >
              <option :value="null">Standalone task</option>
              <option v-for="goal in goals" :key="goal.id" :value="goal.id">
                {{ goal.approvedGoal?.outcome ?? goal.text }}
              </option>
            </select></label
          ><label class="flex flex-col gap-2 text-rune-text-2"
            >Protected storage category<span
              v-if="parentGoal"
              class="rounded-rune-tile bg-rune-surface p-3 text-rune-text-2"
              >{{ parentGoal.class }} · inherited from goal</span
            ><select
              v-else
              v-model="privacy"
              :disabled="!!parentGoal"
              class="rounded-rune-tile bg-rune-raised p-3 text-rune-ink"
            >
              <option
                v-for="item in ['general', 'health', 'mind', 'finance'] as const"
                :key="item"
                :value="item"
              >
                {{ item }}
              </option>
            </select></label
          >
          <p class="m-0 text-rune-heading text-rune-text-2">
            Saved tasks use the protected Life store and need category consent in Settings. A parent
            goal supplies its privacy category.
          </p></template
        ><label class="flex items-center gap-2 text-rune-ink"
          ><input v-model="scheduled" type="checkbox" class="accent-white" />Schedule this
          task</label
        >
        <div v-if="scheduled" class="grid gap-3 sm:grid-cols-2">
          <label class="flex flex-col gap-2 text-rune-text-2"
            >First or next occurrence<input
              v-model="local"
              type="datetime-local"
              required
              class="rounded-rune-tile bg-rune-raised p-3 text-rune-ink [color-scheme:dark]" /></label
          ><label class="flex flex-col gap-2 text-rune-text-2"
            >Repeat<select
              v-model="repeat"
              class="rounded-rune-tile bg-rune-raised p-3 text-rune-ink"
            >
              <option value="once">One time</option>
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
              <option value="monthly">Monthly</option>
            </select></label
          >
          <label v-if="repeat === 'monthly'" class="flex flex-col gap-2 text-rune-text-2"
            >Monthly day<input
              v-model.number="monthlyDay"
              type="number"
              min="1"
              max="31"
              required
              class="rounded-rune-tile bg-rune-surface p-3 text-rune-ink"
          /></label>
          <p class="m-0 text-rune-heading text-rune-text-2 sm:col-span-2">
            Timezone: {{ zone || 'Reading device timezone…' }}. Rune asks before running every
            occurrence.{{ repeat === 'monthly' ? ' Shorter months use their last day.' : '' }}
          </p>
        </div></template
      >
      <p v-else class="m-0 text-rune-heading text-rune-text-2">
        Rune will clarify your current situation and desired outcome before asking you to approve
        the goal.
      </p>
      <RuneButton
        variant="primary"
        type="submit"
        :disabled="pending || (kind === 'task' && scheduled && !zone)"
        class="self-start"
      >
        {{
          pending
            ? 'Saving…'
            : kind === 'goal'
              ? 'Start goal conversation'
              : task
                ? 'Save schedule'
                : 'Save task'
        }}
      </RuneButton>
    </fieldset>
    <p v-if="error" role="alert" class="m-0 text-error">{{ error }}</p>
  </form>
</template>