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
import SettingsArtifacts from './SettingsArtifacts.vue'
import SettingsGeneral from './SettingsGeneral.vue'
import SettingsShortcuts from './SettingsShortcuts.vue'
import TitleBar from './TitleBar.vue'
import SettingsTools from './SettingsTools.vue'

type Section = 'general' | 'shortcuts' | 'agents' | 'artifacts' | 'about' | 'tools'

const agent = useAgent()

const SECTIONS: { id: Section; label: string; subtitle: string; icon: IconName }[] = [
  { id: 'general', label: 'General', subtitle: 'Recording, safety and preferences.', icon: 'tune' },
  {
    id: 'agents',
    label: 'Agents',
    subtitle: 'Connection, models and execution mode.',
    icon: 'plug',
  },
  { id: 'tools', label: 'Tools', subtitle: 'Selected read-only Kit tools.', icon: 'plug' },
  {
    id: 'artifacts',
    label: 'Sessions',
    subtitle: 'Recorded sessions. Pick one to play it in Rune Preview.',
    icon: 'history',
  },
  {
    id: 'shortcuts',
    label: 'Shortcuts',
    subtitle: 'Keys that work from anywhere.',
    icon: 'shortcut',
  },
  { id: 'about', label: 'About', subtitle: 'Installed versions and updates.', icon: 'logo' },
]
const visible = SECTIONS

const asked = new URLSearchParams(location.search).get('section')
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
const apiKey = ref('')
const saved = ref(true)

watch(
  agent.settings,
  (value) => {
    if (value && saved.value) draft.value = { ...value, models: { ...value.models } }
  },
  { immediate: true },
)

const save = useDebounceFn(async () => {
  if (!draft.value) return
  await agent.saveSettings(draft.value)
  saved.value = true
  if (agent.settings.value)
    draft.value = { ...agent.settings.value, models: { ...agent.settings.value.models } }
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

async function commitKey() {
  const key = apiKey.value.trim()
  if (!key) return
  await agent.saveApiKey(key)
  apiKey.value = ''
}

onBeforeUnmount(() => {
  if (!saved.value) void save()
})
</script>

<template>
  <main
    class="flex h-screen flex-col overflow-hidden bg-black text-zinc-100"
    data-testid="settings"
  >
    <TitleBar title="Settings" controls="close" />
    <div class="flex min-h-0 flex-1">
      <nav
        class="flex w-60 shrink-0 flex-col gap-1 border-r border-white/8 bg-[#0c0c0c] p-3"
        aria-label="Settings sections"
      >
        <button
          v-for="item in visible"
          :key="item.id"
          type="button"
          class="flex h-10 w-full cursor-pointer items-center gap-3 rounded-xl border-0 px-3 text-left text-body transition"
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
      </nav>

      <section class="flex min-w-0 flex-1 flex-col overflow-y-auto px-10 py-8">
        <header class="mb-8 flex flex-col gap-1">
          <h1 class="m-0 text-title">{{ current.label }}</h1>
          <p class="m-0 text-label text-zinc-500">{{ current.subtitle }}</p>
        </header>

        <p
          v-if="agent.notice.value"
          class="mt-0 mb-6 text-label"
          :class="agent.noticeKind.value === 'info' ? 'text-zinc-300' : 'text-[#ff6b72]'"
          role="status"
        >
          {{ agent.notice.value }}
        </p>

        <template v-if="draft">
          <SettingsGeneral v-if="section === 'general'" :agent="agent" :draft="draft" />
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
            v-model:api-key="apiKey"
            @commit-key="commitKey"
          />
        </template>
        <SettingsArtifacts v-if="section === 'artifacts'" :agent="agent" />
        <SettingsAbout v-else-if="section === 'about'" :agent="agent" />
      </section>
    </div>
  </main>
</template>