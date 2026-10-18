<script setup lang="ts">
import { nextTick, ref, useTemplateRef, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import BackendBadge from './BackendBadge.vue'
import DevInputLab from './DevInputLab.vue'
import SettingsGroup from './SettingsGroup.vue'
import SettingsRow from './SettingsRow.vue'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const isDev = import.meta.env.DEV
const DEBUG_STATES: AgentState[] = [
  'idle',
  'listening',
  'planning',
  'acting',
  'awaiting_confirm',
  'done',
  'failed',
]

const draft = ref<Settings | null>(null)
const developer = useTemplateRef<HTMLDetailsElement>('developer')
const apiKeyDraft = ref('')

watch(
  agent.settings,
  (value) => {
    if (value) draft.value = { ...value }
  },
  { immediate: true },
)

/** Opening the developer section would otherwise leave its buttons below the fold. */
async function revealDeveloper() {
  if (!developer.value?.open) return
  await nextTick()
  developer.value.scrollIntoView({ block: 'end', behavior: 'smooth' })
}

/** An empty key field means "leave the stored key alone"; removing it is its own action. */
async function save() {
  if (apiKeyDraft.value.trim()) {
    await agent.saveApiKey(apiKeyDraft.value)
    apiKeyDraft.value = ''
  }
  if (draft.value) await agent.saveSettings(draft.value)
}
</script>

<template>
  <section
    class="flex h-full w-full flex-col overflow-hidden rounded-[28px] border border-white/8 bg-[#141414] text-zinc-100"
  >
    <header class="flex items-center gap-1 px-4 pt-4 pb-2">
      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full text-zinc-300 transition hover:bg-white/8 hover:text-white"
        aria-label="Back"
        @click="agent.setMode('open')"
      >
        <AppIcon name="back" />
      </button>
      <div class="flex min-w-0 flex-1 flex-col pl-2">
        <h2 class="text-title">Settings</h2>
        <BackendBadge :status="agent.backend.value" />
      </div>
      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full text-zinc-300 transition hover:bg-white/8 hover:text-white"
        aria-label="Check the backend again"
        @click="agent.refreshBackend()"
      >
        <AppIcon name="refresh" />
      </button>
    </header>

    <div v-if="draft" class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-4 pb-4">
      <SettingsGroup title="Connection">
        <SettingsRow v-model="draft.baseUrl" label="Base URL" icon="link" />
        <SettingsRow
          v-model="apiKeyDraft"
          type="password"
          label="API key"
          icon="key"
          :placeholder="agent.hasApiKey.value ? 'Stored. Type to replace' : 'Not set'"
        />
      </SettingsGroup>

      <button
        v-if="agent.hasApiKey.value"
        type="button"
        class="-mt-2 flex w-fit cursor-pointer items-center gap-2 self-end px-2 text-label text-zinc-500 transition hover:text-[#d71921]"
        @click="agent.saveApiKey('')"
      >
        <AppIcon name="trash" size="sm" />
        Remove stored key
      </button>

      <SettingsGroup title="Models">
        <SettingsRow v-model="draft.visionModel" label="Vision" icon="eye" />
        <SettingsRow v-model="draft.asrModel" label="Speech" icon="mic" />
      </SettingsGroup>

      <SettingsGroup title="Shortcuts">
        <SettingsRow v-model="draft.toggleHotkey" label="Open and close" icon="shortcut" />
        <SettingsRow v-model="draft.stopHotkey" label="Stop" icon="stop" />
      </SettingsGroup>

      <details
        v-if="isDev"
        ref="developer"
        class="rounded-2xl bg-white/5 px-4 py-2"
        @toggle="revealDeveloper"
      >
        <summary class="cursor-pointer text-caption text-zinc-500 uppercase">Developer</summary>
        <div class="flex flex-wrap gap-2 pt-2">
          <button
            v-for="s in DEBUG_STATES"
            :key="s"
            type="button"
            class="h-8 cursor-pointer rounded-full bg-white/8 px-3 text-label text-zinc-300 transition hover:bg-white/15"
            @click="agent.debugSetState(s)"
          >
            {{ s }}
          </button>
        </div>
        <DevInputLab :agent="agent" class="mt-3" />
      </details>
    </div>

    <footer class="p-4 pt-0">
      <button
        type="button"
        class="flex h-10 w-full cursor-pointer items-center justify-center gap-2 rounded-full bg-[#f1f1ef] text-label text-[#111] transition hover:bg-white"
        @click="save"
      >
        <AppIcon name="ok" />
        Save and check
      </button>
    </footer>
  </section>
</template>