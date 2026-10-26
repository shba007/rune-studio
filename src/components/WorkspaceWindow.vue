<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useAgent } from '@/composables/useAgent'
import AppIcon from './AppIcon.vue'
import TaskHistory from './TaskHistory.vue'
import TitleBar from './TitleBar.vue'

type Section = 'goals' | 'tasks'
const agent = useAgent()
const initial = new URLSearchParams(location.search).get('section')
const section = ref<Section>(initial === 'goals' ? 'goals' : 'tasks')
const sections = [
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
    const off = await listen<{ section?: string }>('workspace_section', ({ payload }) => {
      if (payload.section === 'goals' || payload.section === 'tasks')
        section.value = payload.section
    })
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
    <TitleBar title="Goals & Tasks" />
    <div class="flex min-h-0 flex-1">
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
          @click="section = item.id"
        >
          <AppIcon :name="item.id === 'goals' ? 'thinking' : 'history'" />{{ item.label }}
        </button>
        <button
          type="button"
          class="mt-auto flex h-10 cursor-pointer items-center gap-3 rounded-xl px-3 text-left text-body text-zinc-400 hover:bg-white/5 hover:text-zinc-100"
          @click="agent.openSettings()"
        >
          <AppIcon name="tune" />Settings
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
        <TaskHistory v-show="section === 'tasks'" :active="section === 'tasks'" :agent="agent" />
        <div
          v-if="section === 'goals'"
          class="rounded-2xl border border-dotted border-white/15 p-6 text-label text-zinc-400"
        >
          Goal planning is not available yet. Your existing work is available under Tasks.
          <button
            type="button"
            class="mt-4 block cursor-pointer text-zinc-100 underline underline-offset-4"
            @click="section = 'tasks'"
          >
            View tasks
          </button>
        </div>
      </section>
    </div>
  </main>
</template>