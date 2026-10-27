<script setup lang="ts">
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
    class="flex flex-col gap-4 border-t border-white/10 pt-6"
  >
    <header class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <p class="m-0 text-caption uppercase tracking-wider text-zinc-500">From route to action</p>
        <h3 class="mt-1 mb-0 text-body text-zinc-100">Your first week</h3>
      </div>
      <button
        :disabled="busy || entry.status !== 'goal_ready'"
        class="cursor-pointer rounded-full border border-white/20 px-4 py-2 text-label text-zinc-200 disabled:opacity-40"
        @click="$emit('generate')"
      >
        {{
          entry.status === 'task_planning'
            ? 'Drafting…'
            : draft
              ? 'Draft new proposals'
              : 'Draft first-week tasks'
        }}
      </button>
    </header>
    <p class="m-0 max-w-2xl text-label leading-relaxed text-zinc-500">
      Review each task, who will do it and its schedule. Approval creates pending work under this
      goal; it never starts execution or captures a screen. Field edits are saved when you approve.
    </p>
    <template v-if="draft">
      <div class="flex flex-wrap gap-2 text-caption text-zinc-400">
        <span>{{ draft.pathway.title }} ·</span
        ><span
          ><WhenText :iso="draft.startLocal" /> · seven local calendar days · {{ draft.zone }}</span
        >
      </div>
      <p v-if="stale" role="alert" class="m-0 text-label text-[#ff6b72]">
        This draft belongs to an earlier goal or pathway. Generate a new draft before creating
        tasks.
      </p>
      <p class="m-0 max-w-3xl text-label leading-relaxed text-zinc-400">{{ draft.assessment }}</p>
      <p v-if="!draft.tasks.length" class="m-0 text-label text-zinc-500">
        No supported tasks were proposed. Refine the goal or supply the missing context.
      </p>
      <div>
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