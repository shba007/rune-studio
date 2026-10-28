<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { useDebounceFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import type { IconName } from '@/icons'
import type { Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import SettingsAbout from './SettingsAbout.vue'
import SettingsAgents from './SettingsAgents.vue'
import SettingsGeneral from './SettingsGeneral.vue'
import SettingsShortcuts from './SettingsShortcuts.vue'
import TitleBar from './TitleBar.vue'
import SettingsTools from './SettingsTools.vue'
import SettingsConnectionModels from './SettingsConnectionModels.vue'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'
import SettingsGoalContext from './SettingsGoalContext.vue'

type Section =
  | 'general'
  | 'shortcuts'
  | 'agents'
  | 'models'
  | 'capture'
  | 'data'
  | 'developer'
  | 'about'
  | 'tools'

const agent = useAgent()

const SECTIONS: { id: Section; label: string; subtitle: string; icon: IconName }[] = [
  {
    id: 'general',
    label: 'General',
    subtitle: 'Safety, behavior and everyday preferences.',
    icon: 'tune',
  },
  {
    id: 'models',
    label: 'Connections',
    subtitle: 'Manage the model server and its roles.',
    icon: 'plug',
  },
  {
    id: 'agents',
    label: 'Agents',
    subtitle: 'Choose how Rune executes work.',
    icon: 'personality',
  },
  {
    id: 'tools',
    label: 'Tools & connections',
    subtitle: 'Connect to Kit and choose available read-only tools.',
    icon: 'plug',
  },
  {
    id: 'capture',
    label: 'Capture & recording',
    subtitle: 'Saved image quality, recording and Visual Analysis highlights.',
    icon: 'video',
  },
  {
    id: 'data',
    label: 'Data & privacy',
    subtitle: 'Goal classification, protected storage and category consent.',
    icon: 'shield',
  },
  {
    id: 'shortcuts',
    label: 'Shortcuts',
    subtitle: 'Keys that work from anywhere.',
    icon: 'shortcut',
  },
  {
    id: 'developer',
    label: 'Developer',
    subtitle: 'Diagnostics, input tests and animation previews.',
    icon: 'app',
  },
  {
    id: 'about',
    label: 'About',
    subtitle: 'Versions, updates, support and license.',
    icon: 'logo',
  },
]
const search = ref('')
const keywords: Partial<Record<Section, string>> = {
  models:
    'connection backend infer base url api key agent verifier analysis voice microphone transcription decision advisor resolution fast checks',
  capture:
    'screen screenshot image video resolution native highlights entities regions depth landmarks visibility',
  data: 'goal context health mind finance classification consent storage life privacy',
  developer: 'logs debug capture input animation diagnostics',
  general: 'thinking safety approval permission',
  about: 'version update help docs feedback license',
}
const visible = computed(() =>
  SECTIONS.filter((item) => {
    const text = `${item.label} ${item.subtitle} ${keywords[item.id] ?? ''}`.toLowerCase()
    return search.value
      .trim()
      .toLowerCase()
      .split(/\s+/)
      .every((word) => text.includes(word))
  }),
)

const asked = new URLSearchParams(location.search).get('section')
if (['artifacts', 'tasks', 'goals'].includes(asked ?? ''))
  void agent.openWorkspace(asked === 'goals' ? 'goals' : 'tasks')
const section = ref<Section>(
  SECTIONS.some((item) => item.id === asked) ? (asked as Section) : 'general',
)
const current = computed(() => SECTIONS.find((item) => item.id === section.value) ?? SECTIONS[0]!)

let unlisten: (() => void) | null = null
onMounted(async () => {
  try {
    unlisten = await listen<{ section?: Section }>('settings_section', (event) => {
      const requested = event.payload?.section
      if (requested && SECTIONS.some((item) => item.id === requested)) {
        section.value = requested
      }
    })
  } catch {
    unlisten = null
  }
})
onBeforeUnmount(() => unlisten?.())

const draft = ref<Settings | null>(null)
const saved = ref(true)
function copySettings(value: Settings): Settings {
  return {
    ...value,
    models: { ...value.models },
    connections: value.connections?.map((connection) => ({ ...connection })),
    lifeConsents: value.lifeConsents ? { ...value.lifeConsents } : undefined,
    retentionDays: value.retentionDays ? { ...value.retentionDays } : undefined,
  }
}

watch(
  agent.settings,
  (value) => {
    if (value && saved.value) draft.value = copySettings(value)
  },
  { immediate: true },
)

const save = useDebounceFn(async () => {
  if (!draft.value) return
  if ((await agent.saveSettings(draft.value)) === false) return
  saved.value = true
  if (agent.settings.value) draft.value = copySettings(agent.settings.value)
}, 600)

watch(
  draft,
  (value) => {
    if (!value || !agent.settings.value) return
    if (JSON.stringify(value) === JSON.stringify(agent.settings.value)) return
    saved.value = false
    void save()
  },
  { deep: true },
)

onBeforeUnmount(() => {
  if (!saved.value) void save()
})
</script>

<template>
  <main
    class="flex h-screen flex-col overflow-hidden bg-black font-rune-reading text-zinc-100"
    data-testid="settings"
  >
    <TitleBar title="Settings" controls="close" />
    <div class="flex min-h-0 flex-1">
      <nav
        class="flex w-64 shrink-0 flex-col gap-1 border-r border-white/8 bg-[#101010] p-4"
        aria-label="Settings sections"
      >
        <h2 class="mx-rune-2 mt-1 mb-4 font-sans text-rune-subtitle font-medium text-zinc-100">
          Settings
        </h2>
        <label class="relative mb-4 block">
          <AppIcon
            name="search"
            class="pointer-events-none absolute top-rune-2 left-3 text-zinc-500"
          />
          <input
            v-model="search"
            type="search"
            aria-label="Search settings"
            placeholder="Search settings"
            class="h-10 w-full rounded-full bg-white/6 pr-3 pl-10 text-rune-body text-zinc-100 outline-none placeholder:text-zinc-500 focus-visible:ring-1 focus-visible:ring-white/30"
          />
        </label>
        <div class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          <button
            v-for="item in visible"
            :key="item.id"
            type="button"
            class="flex h-10 w-full cursor-pointer items-center gap-rune-2 rounded-xl border-0 px-rune-2 text-left text-rune-body transition"
            :class="
              section === item.id
                ? 'bg-white/10 text-zinc-50'
                : 'bg-transparent text-zinc-400 hover:bg-white/5 hover:text-zinc-100'
            "
            :aria-current="section === item.id ? 'page' : undefined"
            :data-testid="`nav-${item.id}`"
            @click="section = item.id"
          >
            <AppIcon :name="item.icon" />
            {{ item.label }}
          </button>
          <p v-if="!visible.length" role="status" class="px-rune-2 text-rune-body text-zinc-500">
            No matching sections.
          </p>
        </div>
      </nav>

      <section class="flex min-w-0 flex-1 flex-col overflow-y-auto px-8 py-8 lg:px-12">
        <div class="mx-auto w-full max-w-4xl">
          <header class="mb-8 flex flex-col gap-1">
            <h1 class="m-0 font-sans text-rune-title">{{ current.label }}</h1>
            <p class="m-0 text-rune-body text-zinc-500">{{ current.subtitle }}</p>
          </header>

          <p
            v-if="agent.notice.value"
            class="mt-0 mb-6 text-rune-body"
            :class="agent.noticeKind.value === 'info' ? 'text-zinc-300' : 'text-error'"
            role="status"
          >
            {{ agent.notice.value }}
          </p>

          <template v-if="draft">
            <SettingsGeneral
              v-if="section === 'general'"
              :agent="agent"
              :draft="draft"
              view="general"
            />
            <div v-else-if="section === 'models'" class="flex flex-col gap-10">
              <SettingsConnectionModels :agent="agent" :draft="draft">
                <SettingsGeneral :agent="agent" :draft="draft" view="models" />
              </SettingsConnectionModels>
            </div>
            <div v-else-if="section === 'capture'" class="flex flex-col gap-10">
              <SettingsGeneral :agent="agent" :draft="draft" view="capture" />
              <SettingsVisualAnalysis :draft="draft" />
            </div>
            <SettingsGoalContext v-else-if="section === 'data'" :draft="draft" />
            <SettingsGeneral
              v-else-if="section === 'developer'"
              :agent="agent"
              :draft="draft"
              view="developer"
            />
            <SettingsTools
              v-else-if="section === 'tools'"
              :tool-calls="
                agent.backend.value?.models?.find(
                  (model) => model.id === agent.settings.value?.models.agent,
                )?.toolCalls
              "
            />
            <SettingsShortcuts v-else-if="section === 'shortcuts'" :draft="draft" />
            <SettingsAgents
              v-else-if="section === 'agents'"
              :agent="agent"
              :draft="draft"
              execution-only
            />
          </template>
          <SettingsAbout v-if="section === 'about'" :agent="agent" />
        </div>
      </section>
    </div>
  </main>
</template>