<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed, ref } from 'vue'
import { formatDate } from '@vueuse/core'
import type {
  GoalTaskDraft,
  ProposedGoalTask,
  TaskApproval,
  ProposalSchedule,
} from '@/types/decisions'
import AppIcon from './AppIcon.vue'
import HubRow from './HubRow.vue'
import RuneChoice from './RuneChoice.vue'
import RuneSwitch from './RuneSwitch.vue'
const props = defineProps<{
  proposal: ProposedGoalTask
  draft: GoalTaskDraft
  busy: boolean
  stale: boolean
}>()
defineEmits<{ review: [approval: TaskApproval | null]; open: [id: string] }>()
const title = ref(props.proposal.title)
const instructions = ref(props.proposal.instructions)
const mode = ref(props.proposal.executionMode)
const scheduled = ref(!!props.proposal.schedule)
const local = ref(props.proposal.schedule?.local ?? '')
const repeat = ref<ProposalSchedule['repeat']>(props.proposal.schedule?.repeat ?? 'once')
const monthlyDay = ref(
  props.proposal.schedule?.anchorDay ?? (local.value ? Number(local.value.slice(8, 10)) : 1),
)
const maxLocal = computed(() => {
  const end = new Date(props.draft.endLocal)
  end.setDate(end.getDate() - 1)
  return `${formatDate(end, 'YYYY-MM-DD')}T23:59`
})
const approval = (): TaskApproval => ({
  title: title.value,
  instructions: instructions.value,
  executionMode: mode.value,
  schedule: scheduled.value
    ? {
        local: local.value,
        zone: props.draft.zone,
        repeat: repeat.value,
        anchorDay: repeat.value === 'monthly' ? monthlyDay.value : null,
      }
    : null,
})
const modes: { value: ProposedGoalTask['executionMode']; label: string }[] = [
  { value: 'manual', label: 'I will do it · manual activity' },
  { value: 'agent', label: 'Rune · computer/tools' },
  { value: 'analysis', label: 'Visual Analysis · fresh capture selection' },
]
const repeats: { value: ProposalSchedule['repeat']; label: string }[] = [
  { value: 'once', label: 'One time' },
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
  { value: 'monthly', label: 'Monthly' },
]
const field =
  'min-w-0 rounded-rune-tile bg-rune-sunken px-rune-2 font-rune-reading text-rune-body text-rune-ink placeholder:text-rune-text-3 [color-scheme:dark]'
</script>
<template>
  <article
    class="bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
    :aria-label="`Proposed task: ${proposal.title}`"
  >
    <div
      v-if="proposal.createdTaskId || proposal.dismissed"
      class="flex items-center justify-between gap-4 text-rune-body"
    >
      <span :class="proposal.dismissed ? 'text-rune-text-3 line-through' : 'text-rune-text-2'">{{
        proposal.title
      }}</span>
      <RuneButton
        variant="ghost"
        size="sm"
        v-if="proposal.createdTaskId"
        @click="$emit('open', proposal.createdTaskId)"
      >
        Task created →
      </RuneButton>
      <span v-else class="text-rune-caption text-rune-text-3">Dismissed</span>
    </div>
    <details v-else>
      <summary class="flex cursor-pointer list-none items-start gap-rune-2 text-rune-body">
        <AppIcon
          :name="mode === 'manual' ? 'target' : mode === 'analysis' ? 'scan' : 'thinking'"
          class="mt-0.5 shrink-0 text-rune-text-3"
        />
        <span class="flex min-w-0 flex-1 flex-col gap-1"
          ><span class="text-rune-body text-rune-ink">{{ proposal.title }}</span
          ><span class="text-rune-caption text-rune-text-3">{{ proposal.rationale }}</span></span
        >
        <span class="shrink-0 text-rune-caption text-rune-text-2">Review ↓</span>
      </summary>
      <form
        class="mt-rune-2 flex flex-col gap-rune-2"
        @submit.prevent="$emit('review', approval())"
      >
        <fieldset
          :disabled="busy || stale"
          class="m-0 flex min-w-0 flex-col gap-rune-2 p-0 disabled:opacity-50"
        >
          <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
            >Task title<input v-model="title" required maxlength="200" :class="[field, 'h-10']"
          /></label>
          <label class="flex flex-col gap-1 text-rune-caption text-rune-text-3"
            >Instructions<textarea
              v-model="instructions"
              required
              maxlength="4000"
              rows="3"
              :class="[field, 'resize-y py-rune-1']"
            />
          </label>
          <div class="flex flex-col gap-0.5">
            <HubRow label="Who will do it?" class="!bg-rune-raised first:!rounded-t-rune-tile">
              <RuneChoice v-model="mode" label="Who will do it?" :options="modes" menu />
            </HubRow>
            <HubRow label="Include a schedule" class="!bg-rune-raised last:!rounded-b-rune-tile">
              <RuneSwitch v-model="scheduled" label="Include a schedule" />
            </HubRow>
          </div>
          <div v-if="scheduled" class="flex flex-col gap-0.5">
            <HubRow label="First occurrence" class="!bg-rune-raised first:!rounded-t-rune-tile">
              <input
                v-model="local"
                type="datetime-local"
                required
                aria-label="First occurrence"
                :min="`${draft.startLocal.slice(0, 10)}T00:00`"
                :max="maxLocal"
                :class="[field, 'h-10']"
              />
            </HubRow>
            <HubRow label="Repeat" class="!bg-rune-raised">
              <RuneChoice v-model="repeat" label="Repeat" :options="repeats" segments />
            </HubRow>
            <HubRow v-if="repeat === 'monthly'" label="Monthly day" class="!bg-rune-raised">
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
              class="m-0 rounded-b-rune-tile bg-rune-raised px-rune-2 py-rune-1 text-rune-caption text-rune-text-3"
            >
              {{ draft.zone }} · Rune asks before each run.
              {{
                repeat !== 'once'
                  ? 'This recurrence continues beyond the first week until changed or paused.'
                  : ''
              }}
              {{ repeat === 'monthly' ? 'Shorter months use their last day.' : '' }}
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-rune-1">
            <RuneButton variant="primary" size="sm" type="submit">
              Approve &amp; create task
            </RuneButton>
            <RuneButton variant="ghost" size="sm" @click="$emit('review', null)">
              Dismiss proposal
            </RuneButton>
          </div>
        </fieldset>
      </form>
    </details>
  </article>
</template>