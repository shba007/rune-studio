<script setup lang="ts">
import RuneIconButton from './RuneIconButton.vue'
import RuneButton from './RuneButton.vue'
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'
import type { IntakeEntry } from '@/types/decisions'
import type { WorkspaceTask } from '@/types/workspace'
import HubRow from './HubRow.vue'
import RuneChoice from './RuneChoice.vue'
import RuneNotice from './RuneNotice.vue'
import RuneSwitch from './RuneSwitch.vue'
const props = defineProps<{
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
const parentChoice = computed({
  get: () => parent.value ?? '',
  set: (value: string) => (parent.value = value || null),
})
const modes: { value: 'manual' | 'agent' | 'analysis'; label: string }[] = [
  { value: 'manual', label: 'I will do it · track my activity' },
  { value: 'agent', label: 'Rune · execute with computer/tools' },
  { value: 'analysis', label: 'Visual Analysis · choose a fresh capture' },
]
const parentOptions = computed(() => [
  { value: '', label: 'Standalone task' },
  ...props.goals.map((goal) => ({
    value: goal.id,
    label: goal.approvedGoal?.outcome ?? goal.text,
  })),
])
const privacyOptions: { value: IntakeEntry['class']; label: string }[] = [
  { value: 'general', label: 'General' },
  { value: 'health', label: 'Health' },
  { value: 'mind', label: 'Mind' },
  { value: 'finance', label: 'Finance' },
]
const repeats: { value: 'once' | 'daily' | 'weekly' | 'monthly'; label: string }[] = [
  { value: 'once', label: 'One time' },
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
  { value: 'monthly', label: 'Monthly' },
]
const field =
  'min-w-0 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3 [color-scheme:dark]'
onMounted(async () => {
  if (!zone.value)
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
    if (props.task) {
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
    emit('saved', 'task', id)
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
    class="flex flex-col gap-rune-2 rounded-rune-tile bg-rune-sunken p-rune-2"
    @submit.prevent="submit"
  >
    <header class="flex items-center justify-between gap-rune-2">
      <h2 class="m-0 font-sans text-rune-subtitle text-rune-ink">
        {{ task ? 'Schedule task' : legacyRunId ? 'Organize this task' : 'New task' }}
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
    <p v-if="task" class="m-0 text-rune-body text-rune-text-2">{{ task.title }}</p>
    <fieldset :disabled="pending" class="m-0 flex min-w-0 flex-col gap-rune-2 p-0">
      <label
        v-if="!task"
        class="flex flex-col gap-1 rounded-rune-tile bg-rune-surface p-rune-2 text-rune-caption text-rune-text-3"
        >What needs to be done?<textarea
          v-model="title"
          rows="2"
          maxlength="4000"
          required
          :readonly="!!legacyRunId"
          :class="[field, 'resize-y py-rune-1']"
        />
      </label>
      <div v-if="!task" class="flex flex-col gap-0.5">
        <HubRow v-if="!legacyRunId" label="Who will do this task?">
          <RuneChoice
            v-model="executionMode"
            label="Who will do this task?"
            :options="modes"
            menu
          />
        </HubRow>
        <HubRow label="Parent goal">
          <RuneChoice v-model="parentChoice" label="Parent goal" :options="parentOptions" menu />
        </HubRow>
        <HubRow
          label="Protected storage category"
          hint="Saved tasks use the protected Life store and need category consent in Settings. A parent goal supplies its privacy category."
        >
          <span v-if="parentGoal" class="text-rune-body text-rune-text-2"
            >{{ parentGoal.class }} · inherited from goal</span
          >
          <RuneChoice
            v-else
            v-model="privacy"
            label="Protected storage category"
            :options="privacyOptions"
            segments
          />
        </HubRow>
      </div>
      <div class="flex flex-col gap-0.5">
        <HubRow label="Schedule this task">
          <RuneSwitch v-model="scheduled" label="Schedule this task" />
        </HubRow>
        <template v-if="scheduled">
          <HubRow label="First or next occurrence">
            <input
              v-model="local"
              type="datetime-local"
              required
              aria-label="First or next occurrence"
              :class="[field, 'h-10']"
            />
          </HubRow>
          <HubRow label="Repeat">
            <RuneChoice v-model="repeat" label="Repeat" :options="repeats" segments />
          </HubRow>
          <HubRow v-if="repeat === 'monthly'" label="Monthly day">
            <input
              v-model.number="monthlyDay"
              type="number"
              min="1"
              max="31"
              required
              aria-label="Monthly day"
              :class="[field, 'h-10 w-24']"
            />
          </HubRow>
          <p
            class="m-0 rounded-b-rune-tile bg-rune-surface px-rune-2 py-rune-1 text-rune-caption text-rune-text-3"
          >
            Timezone: {{ zone || 'Reading device timezone…' }}. Rune asks before running every
            occurrence.{{ repeat === 'monthly' ? ' Shorter months use their last day.' : '' }}
          </p>
        </template>
      </div>
      <RuneButton
        variant="primary"
        type="submit"
        :disabled="pending || (scheduled && !zone)"
        class="self-start"
      >
        {{ pending ? 'Saving…' : task ? 'Save schedule' : 'Save task' }}
      </RuneButton>
    </fieldset>
    <RuneNotice v-if="error" tone="error">{{ error }}</RuneNotice>
  </form>
</template>