<script setup lang="ts">
import RuneNavItem from './RuneNavItem.vue'
import RuneButton from './RuneButton.vue'
import { listen } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useAgent } from '@/composables/useAgent'
import AppIcon from './AppIcon.vue'
import RuneNotice from './RuneNotice.vue'
import type { IconName } from '@/icons'
import TaskHistory from './TaskHistory.vue'
import TitleBar from './TitleBar.vue'
import FlightPlayer from './FlightPlayer.vue'
import DecisionInbox from './DecisionInbox.vue'
import GoalsTable from './GoalsTable.vue'
import PeekPanel from './PeekPanel.vue'
import RuneChip from './RuneChip.vue'
import { domainOf } from '@/config/domains'
import { useDomainPrefs } from '@/composables/useDomainPrefs'
import PersonalitiesPanel from './PersonalitiesPanel.vue'
import KnowledgeOverview from './KnowledgeOverview.vue'
import type { KnowledgeSection } from '@/config/knowledge'
import { useDecisions } from '@/composables/useDecisions'
import { useWorkspaceTasks } from '@/composables/useWorkspaceTasks'
import NewWorkForm from './NewWorkForm.vue'
import type { WorkspaceTask } from '@/types/workspace'
import type { FlightRun } from '@/config/flight'

type Section = Exclude<KnowledgeSection, 'overview'>
const resolveSection = (value: string | null | undefined): Section | null => {
  if (value === 'overview') return 'goals'
  return value === 'goals' || value === 'tasks' || value === 'personalities' ? value : null
}
const agent = useAgent()
const personalities = ref<InstanceType<typeof PersonalitiesPanel> | null>(null)
const decisions = useDecisions()
const workspace = useWorkspaceTasks()
const approvedGoals = computed(() =>
  decisions.entries.value.filter((goal) => goal.kind === 'goal' && goal.approvedGoal),
)
const focusGoalId = ref<string | null>(null)
const { overrides } = useDomainPrefs()
const focusDomain = computed(() => {
  const entry = decisions.entries.value.find((item) => item.id === focusGoalId.value)
  return entry ? domainOf(entry, overrides.value) : null
})
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
const section = ref<Section>(resolveSection(initial) ?? 'tasks')
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
function navigate(next: KnowledgeSection) {
  previewTask.value = null
  section.value = resolveSection(next) ?? 'tasks'
}
const sections: { id: Section; label: string; icon: IconName }[] = [
  { id: 'tasks', label: 'Tasks', icon: 'history' },
  { id: 'goals', label: 'Goals', icon: 'target' },
  { id: 'personalities', label: 'Personalities', icon: 'personality' },
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
        const target = resolveSection(payload.section)
        if (target) navigate(target)
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
    class="flex h-screen flex-col overflow-hidden bg-black font-rune-reading text-rune-ink"
    data-testid="workspace"
  >
    <TitleBar title="Workspace" />
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
        class="flex w-64 shrink-0 flex-col gap-1 bg-rune-sunken p-4 pt-6"
        aria-label="Workspace sections"
      >
        <RuneNavItem
          v-for="item in sections"
          :key="item.id"
          :selected="section === item.id"
          @click="navigate(item.id)"
        >
          <AppIcon :name="item.icon" />{{ item.label }}
        </RuneNavItem>
      </nav>
      <section class="min-w-0 flex-1 overflow-y-auto px-8 py-8 lg:px-12">
        <div
          class="mx-auto flex w-full flex-col gap-rune-3"
          :class="section === 'goals' ? 'max-w-6xl' : 'max-w-4xl'"
        >
          <header class="flex flex-wrap items-center justify-between gap-rune-2">
            <h1 class="m-0 font-sans text-rune-title">{{ current.label }}</h1>
            <RuneButton
              v-if="section === 'goals' || section === 'tasks'"
              variant="primary"
              @click="creation = { kind: section === 'goals' ? 'goal' : 'task' }"
            >
              + New {{ section === 'goals' ? 'goal' : 'task' }}
            </RuneButton>
            <RuneButton
              v-if="section === 'personalities'"
              variant="primary"
              @click="personalities?.create()"
            >
              + New personality
            </RuneButton>
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
          <RuneNotice v-if="agent.notice.value">{{ agent.notice.value }}</RuneNotice>
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
          <div v-if="section === 'goals'" class="flex flex-col gap-rune-3 text-rune-text-2">
            <RuneNotice v-if="decisions.error.value" tone="error">{{
              decisions.error.value
            }}</RuneNotice>
            <GoalsTable
              :entries="decisions.entries.value"
              :tasks="workspace.tasks.value"
              :selected-id="focusGoalId"
              @open="openGoal"
            />
            <KnowledgeOverview @navigate="navigate" />
          </div>
          <DecisionInbox v-if="section === 'tasks'" quiet :model="decisions" />
          <PersonalitiesPanel v-if="section === 'personalities'" ref="personalities" />
        </div>
      </section>
      <PeekPanel
        v-if="section === 'goals' && focusGoalId"
        label="Goal details"
        @close="focusGoalId = null"
      >
        <template #header>
          <RuneChip v-if="focusDomain">{{ focusDomain.name }}</RuneChip>
        </template>
        <DecisionInbox
          goals-only
          :model="decisions"
          :workspace="workspace"
          :focus-goal-id="focusGoalId"
          @goal-tasks="goalTasks"
          @task="openTask"
          @create-task="(id) => (creation = { kind: 'task', parent: id })"
        />
      </PeekPanel>
    </div>
  </main>
</template>