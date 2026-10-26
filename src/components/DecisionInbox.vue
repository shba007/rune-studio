<script setup lang="ts">
import { computed } from 'vue'
import { useDecisions, type DecisionModel } from '@/composables/useDecisions'
import DecisionCard from './DecisionCard.vue'
import GoalPathways from './GoalPathways.vue'
import GoalSummary from './GoalSummary.vue'
import GoalWorkPanel from './GoalWorkPanel.vue'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
const emit = defineEmits<{
  goalTasks: [id: string]
  task: [id: string]
  createTask: [goalId: string]
}>()
const props = defineProps<{
  compact?: boolean
  goalsOnly?: boolean
  model?: DecisionModel
  workspace?: WorkspaceTasksModel
  focusGoalId?: string | null
}>()
const decisions = props.model ?? useDecisions()
const visible = computed(() =>
  props.compact
    ? decisions.current.value
      ? [decisions.current.value]
      : []
    : decisions.entries.value.filter(
        (e) =>
          (props.goalsOnly ? e.kind === 'goal' : true) &&
          (!props.focusGoalId || e.id === props.focusGoalId),
      ),
)
</script>
<template>
  <div class="flex shrink-0 flex-col gap-3">
    <p v-if="decisions.error.value" role="alert" class="m-0 text-label text-[#ff6b72]">
      {{ decisions.error.value }}
    </p>
    <p v-if="!compact && !visible.length" class="text-label text-zinc-400">
      No goal conversations yet. Describe an outcome in the composer to begin.
    </p>
    <article
      v-for="entry in visible"
      :key="entry.id"
      class="flex min-w-0 flex-col [overflow-wrap:anywhere]"
      :class="
        !compact && entry.kind === 'goal'
          ? 'gap-6 border-b border-white/10 pb-10 mb-8 last:border-0 last:pb-0 last:mb-0'
          : 'gap-2'
      "
    >
      <GoalSummary v-if="!compact && entry.kind === 'goal'" :entry="entry" />
      <GoalWorkPanel
        v-if="!compact && entry.kind === 'goal' && entry.approvedGoal && workspace"
        :entry="entry"
        :workspace="workspace"
        @refresh="decisions.refresh"
        @tasks="(id) => emit('goalTasks', id)"
        @task="(id) => emit('task', id)"
        @create="(id) => emit('createTask', id)"
      />
      <template v-if="compact || entry.kind !== 'goal'">
        <p class="m-0 text-label text-zinc-300 whitespace-pre-wrap">{{ entry.text }}</p>
        <p v-if="entry.kind === 'goal' && entry.category" class="m-0 text-caption text-zinc-500">
          Category: {{ entry.category }}
        </p>
        <p v-if="entry.error" role="alert" class="m-0 text-label text-[#ff6b72]">
          {{ entry.error }}
        </p>
        <div
          v-if="entry.approvedGoal && entry.question"
          class="rounded-2xl border border-white/10 p-4 text-label"
        >
          <p class="mt-0 text-caption text-zinc-400 uppercase">Approved definition</p>
          <dl class="m-0">
            <div v-for="(value, key) in entry.approvedGoal" :key="key" class="mb-2 last:mb-0">
              <dt class="text-caption text-zinc-500 uppercase">{{ key }}</dt>
              <dd class="m-0 text-zinc-200">{{ value }}</dd>
            </div>
          </dl>
          <p class="mb-0 text-caption text-zinc-400">
            Review proposed updates below before approving a replacement.
          </p>
        </div>
        <dl v-if="entry.goal" class="m-0 rounded-2xl border border-white/10 p-4 text-label">
          <div v-for="(value, key) in entry.goal" :key="key" class="mb-2 last:mb-0">
            <dt class="text-caption text-zinc-500 uppercase">{{ key }}</dt>
            <dd class="m-0 text-zinc-200 whitespace-pre-wrap">{{ value }}</dd>
          </div>
        </dl>
      </template>
      <p
        v-if="!compact && entry.kind === 'goal' && entry.error"
        role="alert"
        class="m-0 text-label text-[#ff6b72]"
      >
        {{ entry.error }}
      </p>
      <DecisionCard
        v-if="entry.question"
        :question="entry.question"
        :busy="decisions.pending.value"
        :deferable="compact"
        :cancel-label="entry.approvedGoal ? 'Cancel changes' : 'Cancel request'"
        @answer="decisions.answer"
        @defer="decisions.defer"
      />
      <p
        v-else-if="entry.status !== 'goal_ready'"
        role="status"
        class="m-0 text-label text-zinc-400"
      >
        {{
          entry.status === 'goal_ready'
            ? 'Goal approved. Compare pathways when ready; scheduling is not available yet.'
            : entry.status === 'pathway_planning'
              ? 'Comparing pathways using the approved goal…'
              : entry.status === 'interrupted'
                ? 'Review task history before creating another execution attempt.'
                : 'Understanding your request…'
        }}
      </p>
      <div v-if="entry.status === 'goal_ready'" class="flex flex-wrap items-center gap-3">
        <button
          v-if="entry.status === 'goal_ready' && !compact"
          type="button"
          :disabled="decisions.pending.value"
          class="inline-flex min-h-10 items-center gap-2 self-start cursor-pointer rounded-full bg-zinc-100 px-5 py-2 text-label text-zinc-950 disabled:opacity-40"
          @click="decisions.comparePathways(entry)"
        >
          {{ entry.pathways ? 'Compare new alternatives' : 'Compare pathways' }}
        </button>
        <button
          v-if="entry.status === 'goal_ready'"
          type="button"
          :disabled="decisions.pending.value"
          class="min-h-10 self-start cursor-pointer rounded-full px-4 py-2 text-label text-zinc-400 hover:bg-white/5 hover:text-zinc-100 disabled:opacity-40"
          @click="decisions.refine(entry)"
        >
          Refine goal
        </button>
      </div>
      <GoalPathways
        v-if="!compact && entry.pathways"
        :entry="entry"
        :busy="decisions.pending.value"
        @select="(id) => decisions.selectPathway(entry, id)"
      />
    </article>
  </div>
</template>