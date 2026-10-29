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
</script>
<template>
  <article class="py-rune-3" :aria-label="`Proposed task: ${proposal.title}`">
    <div
      v-if="proposal.createdTaskId || proposal.dismissed"
      class="flex items-center justify-between gap-4 text-rune-body"
    >
      <span :class="proposal.dismissed ? 'text-rune-text-2 line-through' : 'text-rune-text-2'">{{
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
      <span v-else class="text-rune-caption text-rune-text-2">Dismissed</span>
    </div>
    <details v-else>
      <summary class="flex cursor-pointer list-none items-start gap-rune-2 text-rune-body">
        <AppIcon
          :name="mode === 'manual' ? 'target' : mode === 'analysis' ? 'scan' : 'thinking'"
          class="mt-0.5 shrink-0 text-rune-text-2"
        />
        <span class="min-w-0 flex-1"
          ><span class="block text-rune-body text-rune-ink">{{ proposal.title }}</span
          ><span class="mt-1 block leading-relaxed text-rune-text-2">{{
            proposal.rationale
          }}</span></span
        >
        <span class="shrink-0 text-rune-text-2">Review ↓</span>
      </summary>
      <form
        class="mt-rune-3 flex flex-col gap-4 pl-rune-3 text-rune-body"
        @submit.prevent="$emit('review', approval())"
      >
        <fieldset
          :disabled="busy || stale"
          class="m-0 flex min-w-0 flex-col gap-4 p-0 disabled:opacity-50"
        >
          <label class="flex flex-col gap-2 text-rune-text-2"
            >Task title<input
              v-model="title"
              required
              maxlength="200"
              class="rounded-rune-tile bg-rune-surface p-rune-2 text-rune-ink"
          /></label>
          <label class="flex flex-col gap-2 text-rune-text-2"
            >Instructions<textarea
              v-model="instructions"
              required
              maxlength="4000"
              rows="3"
              class="resize-y rounded-rune-tile bg-rune-surface p-rune-2 leading-relaxed text-rune-ink"
            />
          </label>
          <label class="flex flex-col gap-2 text-rune-text-2"
            >Who will do it?<select
              v-model="mode"
              class="rounded-rune-tile bg-rune-raised p-rune-2 text-rune-ink"
            >
              <option value="manual">I will do it · manual activity</option>
              <option value="agent">Rune · computer/tools</option>
              <option value="analysis">Visual Analysis · fresh capture selection</option>
            </select></label
          >
          <label class="flex items-center gap-2 text-rune-text-2"
            ><input v-model="scheduled" type="checkbox" class="accent-white" /> Include a
            schedule</label
          >
          <div v-if="scheduled" class="grid gap-4 sm:grid-cols-2">
            <label class="flex flex-col gap-2 text-rune-text-2"
              >First occurrence<input
                v-model="local"
                type="datetime-local"
                required
                :min="`${draft.startLocal.slice(0, 10)}T00:00`"
                :max="maxLocal"
                class="rounded-rune-tile bg-rune-raised p-rune-2 text-rune-ink [color-scheme:dark]"
            /></label>
            <label class="flex flex-col gap-2 text-rune-text-2"
              >Repeat<select
                v-model="repeat"
                class="rounded-rune-tile bg-rune-raised p-rune-2 text-rune-ink"
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
                class="rounded-rune-tile bg-rune-surface p-rune-2 text-rune-ink"
            /></label>
            <p class="m-0 text-rune-caption leading-relaxed text-rune-text-2 sm:col-span-2">
              {{ draft.zone }} · Rune asks before each run.
              {{
                repeat !== 'once'
                  ? 'This recurrence continues beyond the first week until changed or paused.'
                  : ''
              }}
              {{ repeat === 'monthly' ? 'Shorter months use their last day.' : '' }}
            </p>
          </div>
          <div class="flex flex-wrap gap-4 pt-2">
            <RuneButton variant="primary" type="submit"> Approve &amp; create task </RuneButton>
            <RuneButton variant="ghost" size="sm" @click="$emit('review', null)">
              Dismiss proposal
            </RuneButton>
          </div>
        </fieldset>
      </form>
    </details>
  </article>
</template>