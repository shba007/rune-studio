<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import { SUGGESTIONS } from '@/config/suggestions'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const command = ref('')
const field = useTemplateRef<HTMLTextAreaElement>('field')

const canSend = computed(() => command.value.trim().length > 0 && !agent.isRunning.value)
const isListening = computed(() => agent.state.value === 'listening')
const needsConfirm = computed(() => agent.state.value === 'awaiting_confirm')
const showSuggestions = computed(() => command.value.length === 0 && !agent.isRunning.value)

/** Dot colour of the backend chip: nothing wrong shows green, anything else amber or red. */
const backend = computed(() => {
  const status = agent.backend.value
  if (!status) return { dot: 'bg-zinc-500', text: 'Checking' }
  if (!status.reachable) return { dot: 'bg-[#d71921]', text: 'Offline' }
  if (!status.authOk) return { dot: 'bg-amber-400', text: 'Key rejected' }
  if (status.missingModels.length > 0) return { dot: 'bg-amber-400', text: 'Model missing' }
  if (status.error) return { dot: 'bg-amber-400', text: 'Check backend' }
  return { dot: 'bg-emerald-400', text: `${status.latencyMs ?? '?'} ms` }
})

watch(agent.focusTick, async () => {
  await nextTick()
  field.value?.focus()
})

async function submit() {
  const text = command.value.trim()
  if (!canSend.value) return
  if (await agent.startTask(text)) command.value = ''
}

async function useSuggestion(text: string) {
  command.value = text
  await nextTick()
  field.value?.focus()
}
</script>

<template>
  <div class="flex w-full flex-col gap-2">
    <div
      class="flex h-9 flex-wrap gap-2 overflow-hidden"
      :class="showSuggestions ? 'opacity-100' : 'pointer-events-none opacity-0'"
    >
      <button
        v-for="suggestion in SUGGESTIONS"
        :key="suggestion"
        type="button"
        class="max-w-full cursor-pointer truncate rounded-full border border-white/10 bg-[#141414] px-3.5 py-2 font-mono text-xs text-zinc-300 transition hover:border-white/25 hover:text-white"
        @click="useSuggestion(suggestion)"
      >
        {{ suggestion }}
      </button>
    </div>

    <div data-tauri-drag-region class="rounded-[32px] border border-white/8 bg-[#141414] p-2">
      <div class="flex flex-col rounded-[24px] bg-[#f1f1ef] px-5 pt-4 pb-3 text-[#111]">
        <textarea
          ref="field"
          v-model="command"
          rows="3"
          placeholder="Type or say a command"
          class="h-[84px] w-full resize-none bg-transparent font-mono text-[15px] leading-6 outline-none placeholder:font-dot placeholder:text-lg placeholder:text-[#8a8a86]"
          @keydown.enter.exact.prevent="submit"
        />

        <div class="flex items-center gap-2">
          <button
            type="button"
            class="grid size-9 cursor-pointer place-items-center rounded-full border border-black/15 text-[#111] transition hover:bg-black/6"
            aria-label="Settings"
            @click="agent.setMode('settings')"
          >
            <svg
              viewBox="0 0 24 24"
              class="size-5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              aria-hidden="true"
            >
              <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
              <circle cx="15" cy="7" r="2" />
              <circle cx="9" cy="17" r="2" />
            </svg>
          </button>

          <button
            type="button"
            class="relative grid size-9 cursor-pointer place-items-center rounded-full border transition"
            :class="
              isListening
                ? 'border-[#d71921] bg-[#d71921] text-white'
                : 'border-black/15 text-[#111] hover:bg-black/6'
            "
            :aria-label="isListening ? 'Stop listening' : 'Listen'"
            :aria-pressed="isListening"
            @click="agent.toggleListening()"
          >
            <svg
              viewBox="0 0 24 24"
              class="size-5"
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
          </button>

          <button
            type="button"
            class="ml-auto grid size-11 cursor-pointer place-items-center rounded-full bg-[#111] text-white transition"
            :class="canSend ? 'opacity-100 hover:bg-black' : 'opacity-35'"
            aria-label="Send"
            @click="submit"
          >
            <svg
              viewBox="0 0 24 24"
              class="size-5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path d="M12 19V5M6 11l6-6 6 6" />
            </svg>
          </button>
        </div>
      </div>

      <div class="flex h-12 items-center gap-2 px-2 pt-2">
        <span
          class="truncate rounded-full border border-white/15 px-3 py-1.5 font-mono text-xs text-zinc-300"
        >
          {{ agent.settings.value?.visionModel ?? '…' }}
        </span>
        <span
          class="flex shrink-0 items-center gap-1.5 rounded-full border border-white/15 px-3 py-1.5 font-mono text-xs text-zinc-300"
        >
          <span class="size-2 rounded-full" :class="backend.dot" />
          {{ backend.text }}
        </span>

        <span
          class="ml-auto min-w-0 truncate font-dot text-base"
          :class="
            agent.notice.value
              ? 'text-amber-300'
              : needsConfirm
                ? 'text-[#d71921]'
                : 'text-zinc-300'
          "
        >
          {{ agent.statusText.value }}
        </span>

        <template v-if="needsConfirm">
          <button
            type="button"
            class="cursor-pointer rounded-full border border-[#d71921] px-3 py-1 font-mono text-xs text-[#d71921] hover:bg-[#d71921]/15"
            @click="agent.confirm('pending', true)"
          >
            Yes
          </button>
          <button
            type="button"
            class="cursor-pointer rounded-full border border-zinc-600 px-3 py-1 font-mono text-xs text-zinc-400 hover:bg-white/10"
            @click="agent.confirm('pending', false)"
          >
            No
          </button>
        </template>
        <button
          v-else-if="agent.isRunning.value"
          type="button"
          class="cursor-pointer rounded-full border border-[#d71921] px-3 py-1 font-mono text-xs text-[#d71921] hover:bg-[#d71921]/15"
          @click="agent.stop()"
        >
          Stop
        </button>
      </div>
    </div>
  </div>
</template>