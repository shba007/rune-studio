<script setup lang="ts">
import { nextTick, ref, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import BackendBadge from './BackendBadge.vue'

const agent = useAgent()
const isDev = import.meta.env.DEV

const command = ref('')
const apiKeyDraft = ref('')
const settingsDraft = ref<Settings | null>(null)
const input = useTemplateRef<HTMLInputElement>('input')

const DEBUG_STATES: AgentState[] = [
  'idle',
  'listening',
  'planning',
  'acting',
  'awaiting_confirm',
  'failed',
]

watch(agent.settings, (value) => {
  if (value) settingsDraft.value = { ...value }
})

watch(agent.focusTick, async () => {
  await nextTick()
  input.value?.focus()
})

async function submit() {
  const text = command.value.trim()
  if (!text) return
  await agent.startTask(text)
}

async function saveKey() {
  await agent.saveApiKey(apiKeyDraft.value)
  apiKeyDraft.value = ''
}
</script>

<template>
  <div class="flex h-full flex-col p-2" @keydown.esc="agent.hide()">
    <div
      class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-3xl border border-white/10 bg-zinc-900/95 text-zinc-100 shadow-2xl"
    >
      <div data-tauri-drag-region class="flex h-20 shrink-0 items-center gap-3 px-4">
        <button
          type="button"
          class="grid size-11 shrink-0 place-items-center rounded-full bg-zinc-800 text-lg transition hover:bg-zinc-700"
          :class="{ 'bg-red-600 hover:bg-red-500': agent.state.value === 'listening' }"
          title="Listen (voice arrives in a later milestone)"
          @click="agent.toggleListening()"
        >
          🎙
        </button>

        <div data-tauri-drag-region class="flex min-w-0 flex-1 flex-col gap-1">
          <input
            ref="input"
            v-model="command"
            type="text"
            placeholder="Say or type a command…"
            class="w-full bg-transparent text-base text-zinc-100 outline-none placeholder:text-zinc-500"
            @keydown.enter="submit"
          />
          <div data-tauri-drag-region class="flex items-center gap-3">
            <span class="text-xs font-medium text-zinc-300">{{ agent.stateLabel.value }}</span>
            <BackendBadge :status="agent.backend.value" />
          </div>
        </div>

        <button
          v-if="agent.isRunning.value"
          type="button"
          class="rounded-full bg-red-600 px-3 py-1.5 text-xs font-semibold hover:bg-red-500"
          @click="agent.stop()"
        >
          Stop
        </button>
        <button
          type="button"
          class="grid size-8 shrink-0 place-items-center rounded-full text-zinc-400 transition hover:bg-zinc-800 hover:text-zinc-100"
          :title="agent.expanded.value ? 'Collapse' : 'Expand'"
          @click="agent.setExpanded(!agent.expanded.value)"
        >
          {{ agent.expanded.value ? '⌃' : '⌄' }}
        </button>
      </div>

      <p v-if="agent.notice.value" class="px-5 pb-2 text-xs text-amber-300">
        {{ agent.notice.value }}
      </p>

      <div
        v-if="agent.expanded.value"
        class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto border-t border-white/10 p-5 text-sm"
      >
        <section v-if="settingsDraft" class="grid gap-3">
          <h2 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">rune-infer</h2>
          <label class="grid gap-1">
            <span class="text-xs text-zinc-400">Base URL</span>
            <input
              v-model="settingsDraft.baseUrl"
              class="rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500"
            />
          </label>
          <div class="grid grid-cols-2 gap-3">
            <label class="grid gap-1">
              <span class="text-xs text-zinc-400">Vision model</span>
              <input
                v-model="settingsDraft.visionModel"
                class="rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </label>
            <label class="grid gap-1">
              <span class="text-xs text-zinc-400">Speech model</span>
              <input
                v-model="settingsDraft.asrModel"
                class="rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </label>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <label class="grid gap-1">
              <span class="text-xs text-zinc-400">Summon hotkey</span>
              <input
                v-model="settingsDraft.toggleHotkey"
                class="rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </label>
            <label class="grid gap-1">
              <span class="text-xs text-zinc-400">Stop hotkey</span>
              <input
                v-model="settingsDraft.stopHotkey"
                class="rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500"
              />
            </label>
          </div>
          <div class="flex gap-2">
            <button
              type="button"
              class="rounded-lg bg-zinc-700 px-3 py-1.5 hover:bg-zinc-600"
              @click="agent.saveSettings(settingsDraft)"
            >
              Save and recheck
            </button>
            <button
              type="button"
              class="rounded-lg px-3 py-1.5 text-zinc-300 hover:bg-zinc-800"
              @click="agent.refreshBackend()"
            >
              Recheck
            </button>
          </div>
        </section>

        <section class="grid gap-2">
          <h2 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">API key</h2>
          <div class="flex gap-2">
            <input
              v-model="apiKeyDraft"
              type="password"
              :placeholder="
                agent.hasApiKey.value
                  ? 'Key stored (type to replace, empty to clear)'
                  : 'No key stored'
              "
              class="min-w-0 flex-1 rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500"
            />
            <button
              type="button"
              class="rounded-lg bg-zinc-700 px-3 py-1.5 hover:bg-zinc-600"
              @click="saveKey"
            >
              Save
            </button>
          </div>
        </section>

        <section v-if="isDev" class="grid gap-2">
          <h2 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">
            Dev: force state
          </h2>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="s in DEBUG_STATES"
              :key="s"
              type="button"
              class="rounded-lg bg-zinc-800 px-2.5 py-1 text-xs hover:bg-zinc-700"
              @click="agent.debugSetState(s)"
            >
              {{ s }}
            </button>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>