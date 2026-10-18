<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'

import { useAgent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import BackendBadge from './BackendBadge.vue'
import GlyphOrb from './GlyphOrb.vue'

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
  'done',
  'failed',
]
const FIELD = 'rounded-lg bg-zinc-800 px-3 py-1.5 outline-none focus:ring-1 focus:ring-zinc-500'

const isAmbient = computed(() => agent.mode.value === 'ambient')
const needsConfirm = computed(() => agent.state.value === 'awaiting_confirm')

/** Backend trouble shows as a dot on the disc; a healthy backend shows nothing. */
const backendDot = computed(() => {
  const status = agent.backend.value
  if (!status) return null
  if (!status.reachable) return 'bg-red-500'
  if (!status.authOk || status.missingModels.length > 0 || status.error) return 'bg-amber-400'
  return null
})

watch(agent.settings, (value) => {
  if (value) settingsDraft.value = { ...value }
})

watch(agent.focusTick, async () => {
  await nextTick()
  input.value?.focus()
})

/** Idle: the glyph opens and closes the capsule. While the agent runs it is the Stop button. */
function onGlyph() {
  if (agent.isRunning.value) void agent.stop()
  else void agent.toggleOpen()
}

async function submit() {
  const text = command.value.trim()
  if (text) await agent.startTask(text)
}

async function saveKey() {
  await agent.saveApiKey(apiKeyDraft.value)
  apiKeyDraft.value = ''
}
</script>

<template>
  <div class="flex h-full w-full flex-col items-center gap-2" @keydown.esc="agent.dismiss()">
    <div
      data-tauri-drag-region
      class="flex h-19 shrink-0 items-center gap-2.5 overflow-hidden rounded-full bg-[#141414] p-1.5 transition-[width] duration-300 ease-out"
      :style="{ width: isAmbient ? '76px' : '480px' }"
    >
      <button
        type="button"
        class="relative size-16 shrink-0 cursor-pointer rounded-full bg-[#f1f1ef] outline-none focus-visible:ring-2 focus-visible:ring-white/60"
        :aria-label="agent.isRunning.value ? 'Stop' : isAmbient ? 'Open' : 'Close'"
        @click="onGlyph"
      >
        <GlyphOrb :state="agent.state.value" />
        <span
          v-if="backendDot"
          class="absolute right-0.5 bottom-0.5 size-2.5 rounded-full ring-2 ring-[#f1f1ef]"
          :class="backendDot"
        />
      </button>

      <div
        data-tauri-drag-region
        class="flex min-w-0 flex-1 items-center gap-2 transition-opacity duration-200"
        :class="isAmbient ? 'pointer-events-none opacity-0' : 'opacity-100'"
      >
        <div
          v-if="needsConfirm"
          class="flex min-w-0 flex-1 items-center gap-2 font-mono text-sm text-zinc-100"
        >
          <span class="min-w-0 flex-1 truncate">Needs your OK</span>
          <button
            type="button"
            class="rounded-full border border-[#d71921] px-3 py-1 text-xs text-[#d71921] hover:bg-[#d71921]/15"
            @click="agent.confirm('pending', true)"
          >
            Yes
          </button>
          <button
            type="button"
            class="rounded-full border border-zinc-600 px-3 py-1 text-xs text-zinc-400 hover:bg-white/10"
            @click="agent.confirm('pending', false)"
          >
            No
          </button>
        </div>
        <input
          v-else
          ref="input"
          v-model="command"
          type="text"
          :placeholder="agent.notice.value ?? 'Type or say a command'"
          class="h-11 min-w-0 flex-1 bg-transparent px-1 font-mono text-sm text-zinc-100 outline-none placeholder:text-zinc-500"
          :class="{ 'placeholder:text-amber-300': agent.notice.value }"
          @keydown.enter="submit"
        />
      </div>

      <button
        type="button"
        class="relative grid size-16 shrink-0 cursor-pointer place-items-center rounded-full bg-[#2a2a28] text-zinc-100 outline-none transition-opacity duration-200 hover:bg-[#363633] focus-visible:ring-2 focus-visible:ring-white/60"
        :class="isAmbient ? 'pointer-events-none opacity-0' : 'opacity-100'"
        aria-label="Listen"
        @click="agent.toggleListening()"
      >
        <svg
          viewBox="0 0 24 24"
          class="size-6"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <rect x="9" y="3" width="6" height="11" rx="3" />
          <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v3" />
        </svg>
        <span
          v-if="agent.state.value === 'listening'"
          class="absolute top-3 right-3 size-2 rounded-full bg-[#d71921]"
        />
      </button>
    </div>

    <section
      v-if="agent.mode.value === 'settings'"
      class="flex min-h-0 w-full flex-1 flex-col gap-4 overflow-y-auto rounded-3xl bg-[#141414] p-5 text-sm text-zinc-100"
    >
      <BackendBadge :status="agent.backend.value" />

      <div v-if="settingsDraft" class="grid gap-3">
        <h2 class="text-xs font-semibold tracking-wide text-zinc-400 uppercase">rune-infer</h2>
        <label class="grid gap-1">
          <span class="text-xs text-zinc-400">Base URL</span>
          <input v-model="settingsDraft.baseUrl" :class="FIELD" />
        </label>
        <div class="grid grid-cols-2 gap-3">
          <label class="grid gap-1">
            <span class="text-xs text-zinc-400">Vision model</span>
            <input v-model="settingsDraft.visionModel" :class="FIELD" />
          </label>
          <label class="grid gap-1">
            <span class="text-xs text-zinc-400">Speech model</span>
            <input v-model="settingsDraft.asrModel" :class="FIELD" />
          </label>
          <label class="grid gap-1">
            <span class="text-xs text-zinc-400">Summon hotkey</span>
            <input v-model="settingsDraft.toggleHotkey" :class="FIELD" />
          </label>
          <label class="grid gap-1">
            <span class="text-xs text-zinc-400">Stop hotkey</span>
            <input v-model="settingsDraft.stopHotkey" :class="FIELD" />
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
          <button
            type="button"
            class="ml-auto rounded-lg px-3 py-1.5 text-zinc-300 hover:bg-zinc-800"
            @click="agent.setMode('open')"
          >
            Done
          </button>
        </div>
      </div>

      <div class="grid gap-2">
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
            :class="[FIELD, 'min-w-0 flex-1']"
          />
          <button
            type="button"
            class="rounded-lg bg-zinc-700 px-3 py-1.5 hover:bg-zinc-600"
            @click="saveKey"
          >
            Save
          </button>
        </div>
      </div>

      <div v-if="isDev" class="grid gap-2">
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
      </div>
    </section>
  </div>
</template>