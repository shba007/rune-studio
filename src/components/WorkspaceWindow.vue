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

type Section = KnowledgeSection
const agent = useAgent()
const initial = new URLSearchParams(location.search).get('section')
const section = ref<Section>(
  initial === 'goals' || initial === 'personalities' || initial === 'overview' ? initial : 'tasks',
)
const previewTask = ref<string | null>(new URLSearchParams(location.search).get('run'))
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
  { id: 'goals' as const, label: 'Goals', subtitle: 'Long-term outcomes, pathways and progress.' },
  {
    id: 'tasks' as const,
    label: 'Tasks',
    subtitle: 'Your work, results, analysis and recordings.',
  },
  {
    id: 'personalities' as const,
    label: 'Personalities',
    subtitle: 'Approved coaching styles and their goal categories.',
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
    class="flex h-screen flex-col overflow-hidden bg-black text-zinc-100"
    data-testid="workspace"
  >
    <TitleBar title="Knowledge" />
    <FlightPlayer
      v-if="previewTask"
      :key="previewTask"
      embedded
      :task-id="previewTask"
      @back="navigate('tasks')"
    />
    <div v-show="!previewTask" class="flex min-h-0 flex-1">
      <nav
        class="flex w-48 shrink-0 flex-col gap-1 border-r border-white/8 bg-[#0c0c0c] p-3"
        aria-label="Workspace sections"
      >
        <button
          v-for="item in sections"
          :key="item.id"
          type="button"
          class="flex h-10 cursor-pointer items-center gap-3 rounded-xl px-3 text-left text-body transition"
          :class="
            section === item.id
              ? 'bg-white/10 text-zinc-50'
              : 'text-zinc-400 hover:bg-white/5 hover:text-zinc-100'
          "
          :aria-current="section === item.id ? 'page' : undefined"
          @click="navigate(item.id)"
        >
          <AppIcon :name="item.id === 'goals' ? 'thinking' : 'history'" />{{ item.label }}
        </button>
      </nav>
      <section class="min-w-0 flex-1 overflow-y-auto px-8 py-6">
        <header class="mb-6">
          <h1 class="m-0 text-title">{{ current.label }}</h1>
          <p class="mt-1 text-label text-zinc-500">{{ current.subtitle }}</p>
        </header>
        <p v-if="agent.notice.value" role="alert" class="text-label text-zinc-400">
          {{ agent.notice.value }}
        </p>
        <TaskHistory
          v-show="section === 'tasks'"
          :active="section === 'tasks' && !previewTask"
          :agent="agent"
          :open-task="(id) => (previewTask = id)"
        />
        <div
          v-if="section === 'goals'"
          class="rounded-2xl border border-dotted border-white/15 p-6 text-label text-zinc-400"
        >
          <DecisionInbox goals-only />
          <button
            type="button"
            class="mt-4 block cursor-pointer text-zinc-100 underline underline-offset-4"
            @click="section = 'tasks'"
          >
            View tasks
          </button>
        </div>
        <DecisionInbox v-if="section === 'tasks'" />
        <PersonalitiesPanel v-if="section === 'personalities'" />
        <KnowledgeOverview v-if="section === 'overview'" @navigate="navigate" />
      </section>
    </div>
  </main>
</template>