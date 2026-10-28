<script setup lang="ts">
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
  <article
    class="border-b border-white/10 py-rune-3 last:border-0"
    :aria-label="`Proposed task: ${proposal.title}`"
  >
    <div
      v-if="proposal.createdTaskId || proposal.dismissed"
      class="flex items-center justify-between gap-4 text-rune-body"
    >
      <span :class="proposal.dismissed ? 'text-zinc-500 line-through' : 'text-zinc-300'">{{
        proposal.title
      }}</span>
      <button
        v-if="proposal.createdTaskId"
        class="shrink-0 cursor-pointer text-zinc-200"
        @click="$emit('open', proposal.createdTaskId)"
      >
        Task created →
      </button>
      <span v-else class="text-rune-caption text-zinc-500">Dismissed</span>
    </div>
    <details v-else>
      <summary class="flex cursor-pointer list-none items-start gap-rune-2 text-rune-body">
        <AppIcon
          :name="mode === 'manual' ? 'target' : mode === 'analysis' ? 'scan' : 'thinking'"
          class="mt-0.5 shrink-0 text-zinc-500"
        />
        <span class="min-w-0 flex-1"
          ><span class="block text-rune-body text-zinc-200">{{ proposal.title }}</span
          ><span class="mt-1 block leading-relaxed text-zinc-500">{{
            proposal.rationale
          }}</span></span
        >
        <span class="shrink-0 text-zinc-400">Review ↓</span>
      </summary>
      <form
        class="mt-rune-3 flex flex-col gap-4 pl-rune-3 text-rune-body"
        @submit.prevent="$emit('review', approval())"
      >
        <fieldset
          :disabled="busy || stale"
          class="m-0 flex min-w-0 flex-col gap-4 border-0 p-0 disabled:opacity-50"
        >
          <label class="flex flex-col gap-2 text-zinc-400"
            >Task title<input
              v-model="title"
              required
              maxlength="200"
              class="rounded-xl bg-white/5 p-rune-2 text-zinc-100"
          /></label>
          <label class="flex flex-col gap-2 text-zinc-400"
            >Instructions<textarea
              v-model="instructions"
              required
              maxlength="4000"
              rows="3"
              class="resize-y rounded-xl bg-white/5 p-rune-2 leading-relaxed text-zinc-100"
            />
          </label>
          <label class="flex flex-col gap-2 text-zinc-400"
            >Who will do it?<select
              v-model="mode"
              class="rounded-xl bg-[#191919] p-rune-2 text-zinc-100"
            >
              <option value="manual">I will do it · manual activity</option>
              <option value="agent">Rune · computer/tools</option>
              <option value="analysis">Visual Analysis · fresh capture selection</option>
            </select></label
          >
          <label class="flex items-center gap-2 text-zinc-300"
            ><input v-model="scheduled" type="checkbox" class="accent-white" /> Include a
            schedule</label
          >
          <div v-if="scheduled" class="grid gap-4 sm:grid-cols-2">
            <label class="flex flex-col gap-2 text-zinc-400"
              >First occurrence<input
                v-model="local"
                type="datetime-local"
                required
                :min="`${draft.startLocal.slice(0, 10)}T00:00`"
                :max="maxLocal"
                class="rounded-xl bg-[#191919] p-rune-2 text-zinc-100 [color-scheme:dark]"
            /></label>
            <label class="flex flex-col gap-2 text-zinc-400"
              >Repeat<select
                v-model="repeat"
                class="rounded-xl bg-[#191919] p-rune-2 text-zinc-100"
              >
                <option value="once">One time</option>
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
                <option value="monthly">Monthly</option>
              </select></label
            >
            <label v-if="repeat === 'monthly'" class="flex flex-col gap-2 text-zinc-400"
              >Monthly day<input
                v-model.number="monthlyDay"
                type="number"
                min="1"
                max="31"
                required
                class="rounded-xl bg-white/5 p-rune-2 text-zinc-100"
            /></label>
            <p class="m-0 text-rune-caption leading-relaxed text-zinc-500 sm:col-span-2">
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
            <button
              type="submit"
              class="cursor-pointer rounded-full bg-zinc-100 px-4 py-2 text-zinc-950"
            >
              Approve &amp; create task
            </button>
            <button
              type="button"
              class="cursor-pointer text-zinc-500 hover:text-zinc-200"
              @click="$emit('review', null)"
            >
              Dismiss proposal
            </button>
          </div>
        </fieldset>
      </form>
    </details>
  </article>
</template>