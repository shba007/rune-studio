<script setup lang="ts">
import RuneNavItem from './RuneNavItem.vue'
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
    label: 'Models',
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
    label: 'Kit & tools',
    subtitle: 'Connect to Kit and choose available read-only tools.',
    icon: 'link',
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
const SYSTEM_SECTIONS: Section[] = ['developer', 'about']
const visible = SECTIONS

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
    class="flex h-screen flex-col overflow-hidden bg-black font-rune-reading text-rune-ink"
    data-testid="settings"
  >
    <TitleBar title="Settings" controls="close" />
    <div class="flex min-h-0 flex-1">
      <nav
        class="flex w-64 shrink-0 flex-col gap-1 bg-rune-sunken p-4"
        aria-label="Settings sections"
      >
        <h2 class="mx-rune-2 mt-1 mb-6 font-sans text-rune-subtitle font-medium text-rune-ink">
          Settings
        </h2>
        <div class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          <template v-for="(item, index) in visible" :key="item.id">
            <hr
              v-if="
                SYSTEM_SECTIONS.includes(item.id) &&
                index > 0 &&
                !SYSTEM_SECTIONS.includes(visible[index - 1]!.id)
              "
              class="my-0 h-4 border-0"
              data-testid="nav-divider"
            />
            <RuneNavItem
              :selected="section === item.id"
              :data-testid="`nav-${item.id}`"
              @click="section = item.id"
            >
              <AppIcon :name="item.icon" />
              {{ item.label }}
            </RuneNavItem>
          </template>
        </div>
      </nav>

      <section class="flex min-w-0 flex-1 flex-col overflow-y-auto px-8 py-8 lg:px-12">
        <div class="mx-auto w-full max-w-4xl">
          <header class="mb-8 flex flex-col gap-1">
            <h1 class="m-0 font-sans text-rune-title">{{ current.label }}</h1>
            <p class="m-0 text-rune-body text-rune-text-2">{{ current.subtitle }}</p>
          </header>

          <p
            v-if="agent.notice.value"
            class="mt-0 mb-6 text-rune-body"
            :class="agent.noticeKind.value === 'info' ? 'text-rune-text-2' : 'text-error'"
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