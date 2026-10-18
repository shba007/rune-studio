<script setup lang="ts">
import { ref, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import BackendBadge from './BackendBadge.vue'

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
const FIELD = 'rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500'

const draft = ref<Settings | null>(null)
const apiKeyDraft = ref('')

watch(
  agent.settings,
  (value) => {
    if (value) draft.value = { ...value }
  },
  { immediate: true },
)

async function saveKey() {
  await agent.saveApiKey(apiKeyDraft.value)
  apiKeyDraft.value = ''
}
</script>

<template>
  <section
    class="flex h-full w-full flex-col gap-4 overflow-y-auto rounded-[32px] border border-white/8 bg-[#141414] p-6 text-sm text-zinc-100"
  >
    <div class="flex items-center justify-between">
      <h2 class="font-dot text-lg">Settings</h2>
      <button
        type="button"
        class="cursor-pointer rounded-full border border-white/15 px-3 py-1 font-mono text-xs text-zinc-300 hover:bg-white/10"
        @click="agent.setMode('open')"
      >
        Done
      </button>
    </div>

    <BackendBadge :status="agent.backend.value" />

    <div v-if="draft" class="grid gap-3">
      <h3 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">rune-infer</h3>
      <label class="grid gap-1">
        <span class="text-xs text-zinc-400">Base URL</span>
        <input v-model="draft.baseUrl" :class="FIELD" />
      </label>
      <div class="grid grid-cols-2 gap-3">
        <label class="grid gap-1">
          <span class="text-xs text-zinc-400">Vision model</span>
          <input v-model="draft.visionModel" :class="FIELD" />
        </label>
        <label class="grid gap-1">
          <span class="text-xs text-zinc-400">Speech model</span>
          <input v-model="draft.asrModel" :class="FIELD" />
        </label>
        <label class="grid gap-1">
          <span class="text-xs text-zinc-400">Summon hotkey</span>
          <input v-model="draft.toggleHotkey" :class="FIELD" />
        </label>
        <label class="grid gap-1">
          <span class="text-xs text-zinc-400">Stop hotkey</span>
          <input v-model="draft.stopHotkey" :class="FIELD" />
        </label>
      </div>
      <div class="flex gap-2">
        <button
          type="button"
          class="cursor-pointer rounded-lg bg-zinc-700 px-3 py-1.5 hover:bg-zinc-600"
          @click="agent.saveSettings(draft)"
        >
          Save and recheck
        </button>
        <button
          type="button"
          class="cursor-pointer rounded-lg px-3 py-1.5 text-zinc-300 hover:bg-zinc-800"
          @click="agent.refreshBackend()"
        >
          Recheck
        </button>
      </div>
    </div>

    <div class="grid gap-2">
      <h3 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">API key</h3>
      <div class="flex gap-2">
        <input
          v-model="apiKeyDraft"
          type="password"
          :placeholder="
            agent.hasApiKey.value ? 'Key stored (type to replace, empty to clear)' : 'No key stored'
          "
          :class="[FIELD, 'min-w-0 flex-1']"
        />
        <button
          type="button"
          class="cursor-pointer rounded-lg bg-zinc-700 px-3 py-1.5 hover:bg-zinc-600"
          @click="saveKey"
        >
          Save
        </button>
      </div>
    </div>

    <div v-if="isDev" class="grid gap-2">
      <h3 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">Dev: force state</h3>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="s in DEBUG_STATES"
          :key="s"
          type="button"
          class="cursor-pointer rounded-lg bg-zinc-800 px-2.5 py-1 text-xs hover:bg-zinc-700"
          @click="agent.debugSetState(s)"
        >
          {{ s }}
        </button>
      </div>
    </div>
  </section>
</template>