<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useAgent } from '@/composables/useAgent'
import AppIcon from './AppIcon.vue'
import TaskHistory from './TaskHistory.vue'
import TitleBar from './TitleBar.vue'
import FlightPlayer from './FlightPlayer.vue'
import DecisionInbox from './DecisionInbox.vue'
import PersonalitiesPanel from './PersonalitiesPanel.vue'
import KnowledgeOverview from './KnowledgeOverview.vue'
import type { KnowledgeSection } from '@/config/knowledge'
import { useDecisions } from '@/composables/useDecisions'
import { useWorkspaceTasks } from '@/composables/useWorkspaceTasks'
import NewWorkForm from './NewWorkForm.vue'
import type { WorkspaceTask } from '@/types/workspace'
import type { FlightRun } from '@/config/flight'

type Section = KnowledgeSection
const agent = useAgent()
const personalities = ref<InstanceType<typeof PersonalitiesPanel> | null>(null)
const decisions = useDecisions()
const workspace = useWorkspaceTasks()
const approvedGoals = computed(() =>
  decisions.entries.value.filter((goal) => goal.kind === 'goal' && goal.approvedGoal),
)
const focusGoalId = ref<string | null>(null)
const parentFilter = ref<string | null>(null)
const focusTaskId = ref<string | null>(null)
const creation = ref<{
  kind: 'goal' | 'task'
  parent?: string | null
  task?: WorkspaceTask
  run?: FlightRun
} | null>(null)
function openGoal(id: string) {
  section.value = 'goals'
  previewTask.value = null
  focusGoalId.value = id
}
function goalTasks(id: string) {
  section.value = 'tasks'
  previewTask.value = null
  parentFilter.value = id
  focusTaskId.value = null
}
function openTask(id: string) {
  section.value = 'tasks'
  if (workspace.tasks.value.some((task) => task.id === id)) {
    parentFilter.value = null
    focusTaskId.value = id
    previewTask.value = null
  } else previewTask.value = id
}
async function created(kind: 'goal' | 'task', id?: string) {
  creation.value = null
  section.value = kind === 'goal' ? 'goals' : 'tasks'
  focusGoalId.value = kind === 'goal' ? (id ?? null) : null
  focusTaskId.value = kind === 'task' ? (id ?? null) : null
  parentFilter.value = null
  await Promise.all([decisions.refresh(), workspace.refresh()])
}
const initial = new URLSearchParams(location.search).get('section')
const section = ref<Section>(
  initial === 'goals' || initial === 'personalities' || initial === 'overview' ? initial : 'tasks',
)
const previewTask = ref<string | null>(new URLSearchParams(location.search).get('run'))
const previewParent = computed(() => {
  const task = workspace.tasks.value.find(
    (task) =>
      task.legacyTaskId === previewTask.value ||
      task.occurrences.some((o) => o.runId === previewTask.value),
  )
  const goal = approvedGoals.value.find((goal) => goal.id === task?.parentGoalId)
  return goal ? { id: goal.id, title: goal.approvedGoal?.outcome ?? goal.text } : null
})
function navigate(next: Section) {
  previewTask.value = null
  section.value = next
}
const sections = [
  {
    id: 'overview' as const,
    label: 'Overview',
    subtitle: 'Understand Rune, explore its connections, and see what works today.',
  },
  {
    id: 'personalities' as const,
    label: 'Personalities',
    subtitle: 'Choose the style that supports each goal.',
  },
  { id: 'goals' as const, label: 'Goals', subtitle: 'Long-term outcomes, pathways and progress.' },
  {
    id: 'tasks' as const,
    label: 'Tasks',
    subtitle: 'Your work, results, analysis and recordings.',
  },
]
const current = computed(() => sections.find((item) => item.id === section.value)!)
let unlisten: (() => void) | undefined
let disposed = false
onMounted(async () => {
  try {
    const off = await listen<{ section?: string; runId?: string }>(
      'workspace_section',
      ({ payload }) => {
        if (payload.runId) {
          section.value = 'tasks'
          previewTask.value = payload.runId
          return
        }
        if (
          payload.section === 'goals' ||
          payload.section === 'tasks' ||
          payload.section === 'personalities' ||
          payload.section === 'overview'
        )
          navigate(payload.section)
      },
    )
    if (disposed) off()
    else unlisten = off
  } catch {
    // The workspace still supports local navigation if event registration fails.
  }
})
onBeforeUnmount(() => {
  disposed = true
  unlisten?.()
})
</script>

<template>
  <main
    class="flex h-screen flex-col overflow-hidden bg-black font-rune-reading text-zinc-100"
    data-testid="workspace"
  >
    <TitleBar title="Knowledge" />
    <FlightPlayer
      v-if="previewTask"
      :key="previewTask"
      embedded
      :task-id="previewTask"
      :parent-goal="previewParent"
      @goal="openGoal"
      @back="navigate('tasks')"
    />
    <div v-show="!previewTask" class="flex min-h-0 flex-1">
      <nav
        class="flex w-48 shrink-0 flex-col gap-rune-detail border-r border-white/8 bg-[#0c0c0c] p-rune-2"
        aria-label="Workspace sections"
      >
        <button
          v-for="item in sections"
          :key="item.id"
          type="button"
          class="flex min-h-10 cursor-pointer items-center gap-rune-1 rounded-rune-control px-rune-1 text-left text-rune-body transition focus-visible:outline-2 focus-visible:outline-white/60"
          :class="
            section === item.id
              ? 'bg-white/10 text-zinc-50'
              : 'text-zinc-400 hover:bg-white/5 hover:text-zinc-100'
          "
          :aria-current="section === item.id ? 'page' : undefined"
          @click="navigate(item.id)"
        >
          <AppIcon
            :name="
              item.id === 'personalities'
                ? 'personality'
                : item.id === 'overview'
                  ? 'graph'
                  : item.id === 'goals'
                    ? 'target'
                    : 'history'
            "
          />{{ item.label }}
        </button>
      </nav>
      <section class="min-w-0 flex-1 overflow-y-auto px-8 py-6">
        <header class="mb-6">
          <div class="flex flex-wrap items-center justify-between gap-rune-2">
            <h1 class="m-0 font-sans text-rune-title">{{ current.label }}</h1>
            <button
              v-if="section === 'goals' || section === 'tasks'"
              type="button"
              class="cursor-pointer rounded-full bg-zinc-100 px-rune-2 py-rune-1 text-rune-body text-zinc-950"
              @click="creation = { kind: section === 'goals' ? 'goal' : 'task' }"
            >
              + New {{ section === 'goals' ? 'goal' : 'task' }}
            </button>
            <button
              v-if="section === 'personalities'"
              type="button"
              class="cursor-pointer rounded-full bg-zinc-100 px-rune-2 py-rune-1 text-rune-body text-zinc-950"
              @click="personalities?.create()"
            >
              + New personality
            </button>
          </div>
          <p class="mt-rune-detail text-rune-body text-zinc-500">{{ current.subtitle }}</p>
        </header>
        <NewWorkForm
          v-if="creation"
          :key="`${creation.kind}-${creation.task?.id ?? creation.run?.id ?? creation.parent ?? 'new'}`"
          :kind="creation.kind"
          :goals="approvedGoals"
          :parent-goal-id="creation.parent"
          :task="creation.task"
          :legacy-run-id="creation.run?.id"
          :initial-title="creation.run?.task"
          @close="creation = null"
          @saved="created"
        />
        <p v-if="agent.notice.value" role="alert" class="text-rune-body text-zinc-400">
          {{ agent.notice.value }}
        </p>
        <TaskHistory
          v-show="section === 'tasks'"
          :active="section === 'tasks' && !previewTask"
          :agent="agent"
          :open-task="openTask"
          :workspace="workspace"
          :goals="approvedGoals"
          :parent-filter="parentFilter"
          :focus-task-id="focusTaskId"
          @goal="openGoal"
          @schedule="(task) => (creation = { kind: 'task', task })"
          @organize="(run) => (creation = { kind: 'task', run })"
          @clear="parentFilter = null"
        />
        <div v-if="section === 'goals'" class="text-zinc-400">
          <button
            v-if="focusGoalId"
            class="mb-rune-3 cursor-pointer text-rune-body text-zinc-400"
            @click="focusGoalId = null"
          >
            ← All goals
          </button>
          <DecisionInbox
            goals-only
            :model="decisions"
            :workspace="workspace"
            :focus-goal-id="focusGoalId"
            @goal-tasks="goalTasks"
            @task="openTask"
            @create-task="(id) => (creation = { kind: 'task', parent: id })"
          />
          <button
            type="button"
            class="mt-8 block cursor-pointer text-rune-body text-zinc-500 hover:text-zinc-200"
            @click="section = 'tasks'"
          >
            View tasks
          </button>
        </div>
        <DecisionInbox v-if="section === 'tasks'" :model="decisions" />
        <PersonalitiesPanel v-if="section === 'personalities'" ref="personalities" />
        <KnowledgeOverview v-if="section === 'overview'" @navigate="navigate" />
      </section>
    </div>
  </main>
</template>