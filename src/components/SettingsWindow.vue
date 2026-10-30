<script setup lang="ts">
import RuneNavItem from './RuneNavItem.vue'
import { listen } from '@tauri-apps/api/event'
import { useDebounceFn } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import type { IconName } from '@/icons'
import type { Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import SettingsAbout from './SettingsAbout.vue'
import SettingsAgents from './SettingsAgents.vue'
import SettingsGeneral from './SettingsGeneral.vue'
import SettingsShortcuts from './SettingsShortcuts.vue'
import { useNativeBlur } from '@/composables/useNativeBlur'
import { useSectionHistory } from '@/composables/useSectionHistory'
import TitleBar from './TitleBar.vue'
import SettingsTools from './SettingsTools.vue'
import SettingsConnectionModels from './SettingsConnectionModels.vue'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'
import SettingsGoalContext from './SettingsGoalContext.vue'

type Section = 'general' | 'models' | 'capture' | 'developer' | 'about'

const agent = useAgent()

const SECTIONS: { id: Section; label: string; icon: IconName }[] = [
  { id: 'general', label: 'General', icon: 'tune' },
  { id: 'models', label: 'Models & tools', icon: 'plug' },
  { id: 'capture', label: 'Capture & privacy', icon: 'video' },
  { id: 'developer', label: 'Developer', icon: 'app' },
  { id: 'about', label: 'About', icon: 'logo' },
]
const ALIASES: Record<string, Section> = {
  agents: 'general',
  shortcuts: 'general',
  tools: 'models',
  data: 'capture',
}
const resolveSection = (id: string | null | undefined): Section | null => {
  const key = id ?? ''
  if (SECTIONS.some((item) => item.id === key)) return key as Section
  return ALIASES[key] ?? null
}
const SYSTEM_SECTIONS: Section[] = ['developer', 'about']
const visible = SECTIONS

const asked = new URLSearchParams(location.search).get('section')
if (['artifacts', 'tasks', 'goals'].includes(asked ?? ''))
  void agent.openWorkspace(asked === 'goals' ? 'goals' : 'tasks')
const section = ref<Section>(resolveSection(asked) ?? 'general')
const sidebarOpen = ref(true)
const history = useSectionHistory(section)
const windowRoot = useTemplateRef<HTMLElement>('windowRoot')
useNativeBlur(windowRoot, true)
const current = computed(() => SECTIONS.find((item) => item.id === section.value) ?? SECTIONS[0]!)

let unlisten: (() => void) | null = null
onMounted(async () => {
  try {
    unlisten = await listen<{ section?: string }>('settings_section', (event) => {
      const requested = event.payload?.section
      const target = resolveSection(requested)
      if (target) section.value = target
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
    ref="windowRoot"
    class="flex h-screen flex-col overflow-hidden font-rune-reading text-rune-ink"
    data-testid="settings"
  >
    <TitleBar
      sidebar
      title="Settings"
      controls="close"
      :collapsed="!sidebarOpen"
      :menu="visible"
      :can-back="history.canBack.value"
      :can-forward="history.canForward.value"
      @toggle="sidebarOpen = !sidebarOpen"
      @back="history.back()"
      @forward="history.forward()"
      @select="section = $event as Section"
    />
    <div class="flex min-h-0 flex-1">
      <nav
        v-show="sidebarOpen"
        class="glass flex w-64 shrink-0 flex-col gap-1 bg-rune-sunken p-4 pt-2"
        aria-label="Settings sections"
      >
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
          <header class="mb-8">
            <h1 class="m-0 font-sans text-rune-title">{{ current.label }}</h1>
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
            <div v-if="section === 'general'" class="flex flex-col gap-8">
              <SettingsGeneral :agent="agent" :draft="draft" view="general" />
              <SettingsAgents :agent="agent" :draft="draft" execution-only />
              <SettingsShortcuts :draft="draft" />
            </div>
            <div v-else-if="section === 'models'" class="flex flex-col gap-8">
              <SettingsConnectionModels :agent="agent" :draft="draft">
                <SettingsGeneral :agent="agent" :draft="draft" view="models" />
              </SettingsConnectionModels>
              <SettingsTools
                :tool-calls="
                  agent.backend.value?.models?.find(
                    (model) => model.id === agent.settings.value?.models.agent,
                  )?.toolCalls
                "
              />
            </div>
            <div v-else-if="section === 'capture'" class="flex flex-col gap-8">
              <SettingsGeneral :agent="agent" :draft="draft" view="capture" />
              <SettingsVisualAnalysis :draft="draft" />
              <SettingsGoalContext :draft="draft" />
            </div>
            <SettingsGeneral
              v-else-if="section === 'developer'"
              :agent="agent"
              :draft="draft"
              view="developer"
            />
          </template>
          <SettingsAbout v-if="section === 'about'" :agent="agent" />
        </div>
      </section>
    </div>
  </main>
</template>