<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed } from 'vue'
import { useDecisions, type DecisionModel } from '@/composables/useDecisions'
import { goalStatus, goalTitle } from '@/config/goalStatus'
import DecisionCard from './DecisionCard.vue'
import GoalPathways from './GoalPathways.vue'
import GoalSummary from './GoalSummary.vue'
import GoalWorkPanel from './GoalWorkPanel.vue'
import GoalTaskDraft from './GoalTaskDraft.vue'
import WorkEmpty from './WorkEmpty.vue'
import WorkRow from './WorkRow.vue'
import type { WorkspaceTasksModel } from '@/composables/useWorkspaceTasks'
import type { IntakeEntry } from '@/types/decisions'
const emit = defineEmits<{
  goalTasks: [id: string]
  task: [id: string]
  createTask: [goalId: string]
  open: [id: string]
}>()
const props = defineProps<{
  compact?: boolean
  goalsOnly?: boolean
  quiet?: boolean
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
          (!props.quiet || !!e.question) &&
          (!props.focusGoalId || e.id === props.focusGoalId),
      ),
)
const brief = computed(() => props.compact || props.quiet)
const listing = computed(() => !props.compact && props.goalsOnly && !props.focusGoalId)
const rowMeta = (entry: IntakeEntry) => {
  const goal = entry.goal ?? entry.approvedGoal
  return [entry.category, goalStatus(entry).label, goal?.horizon].filter(Boolean).join(' · ')
}
</script>
<template>
  <div class="flex shrink-0 flex-col gap-3">
    <p v-if="decisions.error.value" role="alert" class="m-0 text-rune-caption text-error">
      {{ decisions.error.value }}
    </p>
    <WorkEmpty
      v-if="!compact && !quiet && !visible.length"
      title="No goals yet"
      hint="Use + New goal, or type an outcome in the Rune pill."
    />
    <div v-if="listing && visible.length" class="flex flex-col gap-0.5" aria-label="Goals">
      <WorkRow
        v-for="entry in visible"
        :key="entry.id"
        :data-goal="entry.id"
        :title="goalTitle(entry)"
        :meta="rowMeta(entry)"
        :state="goalStatus(entry).tone"
        interactive
        @open="emit('open', entry.id)"
      />
    </div>
    <article
      v-for="entry in listing ? [] : visible"
      :key="entry.id"
      class="flex min-w-0 flex-col [overflow-wrap:anywhere]"
      :class="!brief && entry.kind === 'goal' ? 'gap-6 pb-10 mb-8 last:pb-0 last:mb-0' : 'gap-2'"
    >
      <GoalSummary v-if="!brief && entry.kind === 'goal'" :entry="entry" />
      <GoalWorkPanel
        v-if="!brief && entry.kind === 'goal' && entry.approvedGoal && workspace"
        :entry="entry"
        :workspace="workspace"
        @refresh="decisions.refresh"
        @tasks="(id) => emit('goalTasks', id)"
        @task="(id) => emit('task', id)"
        @create="(id) => emit('createTask', id)"
      />
      <template v-if="brief || entry.kind !== 'goal'">
        <p class="m-0 text-rune-caption text-rune-text-2 whitespace-pre-wrap">{{ entry.text }}</p>
        <p
          v-if="entry.kind === 'goal' && entry.category"
          class="m-0 text-rune-heading text-rune-text-2"
        >
          Category: {{ entry.category }}
        </p>
        <p v-if="entry.error" role="alert" class="m-0 text-rune-caption text-error">
          {{ entry.error }}
        </p>
        <div
          v-if="entry.approvedGoal && entry.question"
          class="rounded-rune-tile p-4 text-rune-caption bg-rune-raised"
        >
          <p class="mt-0 text-rune-heading text-rune-text-2 uppercase">Approved definition</p>
          <dl class="m-0">
            <div v-for="(value, key) in entry.approvedGoal" :key="key" class="mb-2 last:mb-0">
              <dt class="text-rune-heading text-rune-text-2 uppercase">{{ key }}</dt>
              <dd class="m-0 text-rune-ink">{{ value }}</dd>
            </div>
          </dl>
          <p class="mb-0 text-rune-heading text-rune-text-2">
            Review proposed updates below before approving a replacement.
          </p>
        </div>
        <dl v-if="entry.goal" class="m-0 rounded-rune-tile p-4 text-rune-caption bg-rune-raised">
          <div v-for="(value, key) in entry.goal" :key="key" class="mb-2 last:mb-0">
            <dt class="text-rune-heading text-rune-text-2 uppercase">{{ key }}</dt>
            <dd class="m-0 text-rune-ink whitespace-pre-wrap">{{ value }}</dd>
          </div>
        </dl>
      </template>
      <p
        v-if="!brief && entry.kind === 'goal' && entry.error"
        role="alert"
        class="m-0 text-rune-caption text-error"
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
        class="m-0 text-rune-caption text-rune-text-2"
      >
        {{
          entry.status === 'domain_planning'
            ? 'Suggesting a domain for you to confirm…'
            : entry.status === 'goal_ready'
              ? 'Goal approved. Compare pathways or review linked tasks.'
              : entry.status === 'pathway_planning'
                ? 'Comparing pathways using the approved goal…'
                : entry.status === 'task_planning'
                  ? 'Drafting your first week using the preferred pathway…'
                  : entry.status === 'interrupted'
                    ? 'Review task history before creating another execution attempt.'
                    : 'Understanding your request…'
        }}
      </p>
      <div v-if="entry.status === 'goal_ready'" class="flex flex-wrap items-center gap-3">
        <RuneButton
          variant="primary"
          size="sm"
          v-if="entry.status === 'goal_ready' && !brief"
          :disabled="decisions.pending.value"
          class="self-start"
          @click="decisions.comparePathways(entry)"
        >
          {{ entry.pathways ? 'Compare new alternatives' : 'Compare pathways' }}
        </RuneButton>
        <RuneButton
          variant="ghost"
          size="sm"
          v-if="entry.status === 'goal_ready'"
          :disabled="decisions.pending.value"
          class="self-start"
          @click="decisions.refine(entry)"
        >
          Refine goal
        </RuneButton>
      </div>
      <GoalPathways
        v-if="!brief && entry.pathways"
        :entry="entry"
        :busy="decisions.pending.value"
        @select="(id) => decisions.selectPathway(entry, id)"
      />
      <GoalTaskDraft
        v-if="!brief && entry.pathways?.approved"
        :entry="entry"
        :busy="decisions.pending.value"
        @generate="decisions.proposeTasks(entry)"
        @review="(id, approval) => decisions.reviewTask(entry, id, approval)"
        @open="(id) => emit('task', id)"
      />
    </article>
  </div>
</template>