<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed } from 'vue'
import type { IntakeEntry, TaskApproval } from '@/types/decisions'
import GoalTaskProposal from './GoalTaskProposal.vue'
import WhenText from './WhenText.vue'
const props = defineProps<{ entry: IntakeEntry; busy: boolean }>()
defineEmits<{
  generate: []
  review: [id: string, approval: TaskApproval | null]
  open: [id: string]
}>()
const draft = computed(() => props.entry.taskProposals)
const stale = computed(
  () =>
    !!draft.value &&
    (JSON.stringify(draft.value.goal) !== JSON.stringify(props.entry.approvedGoal) ||
      JSON.stringify(draft.value.pathway) !== JSON.stringify(props.entry.pathways?.approved)),
)
</script>
<template>
  <section
    v-if="entry.pathways?.approved"
    aria-label="First-week task planning"
    class="flex flex-col gap-rune-2"
  >
    <header class="flex flex-wrap items-end justify-between gap-rune-2 px-rune-2">
      <div class="flex flex-col gap-1">
        <h3 class="m-0 font-sans text-rune-heading text-rune-text-3 uppercase">
          From route to action
        </h3>
        <p class="m-0 font-sans text-rune-subtitle text-rune-ink">Your first week</p>
      </div>
      <RuneButton
        variant="secondary"
        size="sm"
        :disabled="busy || entry.status !== 'goal_ready'"
        @click="$emit('generate')"
      >
        {{
          entry.status === 'task_planning'
            ? 'Drafting…'
            : draft
              ? 'Draft new proposals'
              : 'Draft first-week tasks'
        }}
      </RuneButton>
    </header>
    <p class="m-0 max-w-2xl px-rune-2 text-rune-caption text-rune-text-3">
      Review each task, who will do it and its schedule. Approval creates pending work under this
      goal; it never starts execution or captures a screen. Field edits are saved when you approve.
    </p>
    <template v-if="draft">
      <div class="flex flex-wrap gap-2 px-rune-2 text-rune-caption text-rune-text-2">
        <span>{{ draft.pathway.title }} ·</span
        ><span
          ><WhenText :iso="draft.startLocal" /> · seven local calendar days · {{ draft.zone }}</span
        >
      </div>
      <p v-if="stale" role="alert" class="m-0 px-rune-2 text-rune-body text-error">
        This draft belongs to an earlier goal or pathway. Generate a new draft before creating
        tasks.
      </p>
      <p class="m-0 max-w-3xl px-rune-2 text-rune-body text-rune-text-2">
        {{ draft.assessment }}
      </p>
      <p v-if="!draft.tasks.length" class="m-0 px-rune-2 text-rune-body text-rune-text-2">
        No supported tasks were proposed. Refine the goal or supply the missing context.
      </p>
      <div class="flex flex-col gap-0.5">
        <GoalTaskProposal
          v-for="task in draft.tasks"
          :key="`${draft.id}-${task.id}`"
          :proposal="task"
          :draft="draft"
          :busy="busy || entry.status !== 'goal_ready'"
          :stale="stale"
          @review="(approval) => $emit('review', task.id, approval)"
          @open="(id) => $emit('open', id)"
        />
      </div>
    </template>
  </section>
</template>