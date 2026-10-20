<script setup lang="ts">
import { useFocus, useTextareaAutosize } from '@vueuse/core'
import { computed, nextTick, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

import AppIcon from './AppIcon.vue'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

/** The field grows with its text (up to a few lines), and the card with it. */
const { textarea: field, input: command } = useTextareaAutosize()
const { focused } = useFocus(field)

const hasText = computed(() => command.value.trim().length > 0)
const isListening = computed(() => agent.state.value === 'listening')
const needsConfirm = computed(() => agent.state.value === 'awaiting_confirm')

/**
 * One button on the right, always the most useful thing to do next: stop while a task runs,
 * send once there is text, otherwise talk. Its look is the same in every case, only the colour
 * and the icon change.
 */
const action = computed(() => {
  if (agent.isRunning.value)
    return { icon: 'stop', label: 'Stop', tone: 'red', run: () => agent.stop() } as const
  if (hasText.value) return { icon: 'send', label: 'Send', tone: 'dark', run: submit } as const
  return {
    icon: 'mic',
    label: isListening.value ? 'Stop listening' : 'Listen',
    tone: isListening.value ? 'red' : 'dark',
    run: () => agent.toggleListening(),
  } as const
})

/** The field says what the app is doing, so the card never looks idle while it is not. */
const offline = computed(() => agent.health.value.tone === 'down')
const placeholder = computed(() => {
  if (needsConfirm.value) return 'Needs your OK'
  if (agent.isRunning.value) return 'Working. Stop to cancel'
  if (isListening.value) return 'Listening…'
  if (offline.value) return 'Model server offline'
  return 'Type or say a task'
})

watch(agent.focusTick, async () => {
  await nextTick()
  focused.value = true
})

async function submit() {
  const text = command.value.trim()
  if (!text || agent.isRunning.value) return
  if (await agent.startTask(text)) command.value = ''
}
</script>

<template>
  <!-- One row, the same height as the orb (64 px). It grows upward as the text gets longer. -->
  <div
    data-tauri-drag-region
    class="flex w-full flex-col gap-1 rounded-[32px] bg-[#141414] p-2 shadow-[inset_0_0_0_1px_rgb(255_255_255/0.08)]"
  >
    <!-- Notices (a refused task, a mic problem, a failed check) get a line of their own: they can be long. -->
    <p
      v-if="agent.notice.value"
      class="m-0 px-3 pt-1 text-label text-[#ff6b72]"
      role="status"
      aria-live="polite"
    >
      {{ agent.notice.value }}
    </p>
    <!-- What the card is doing, for screen readers: Thinking, Acting, Done. -->
    <p class="sr-only" role="status" aria-live="polite">{{ agent.statusText.value }}</p>
    <!-- Light by default (a pale field in the dark card); fully dark when the system is in dark mode. -->
    <div
      class="flex w-full items-end gap-1 rounded-[24px] bg-[#f1f1ef] p-2 text-[#111] dark:bg-[#1f1f1f] dark:text-zinc-100"
    >
      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full border border-black/15 text-[#111] transition hover:bg-black/6 dark:border-white/15 dark:text-zinc-200 dark:hover:bg-white/8"
        aria-label="Settings"
        @click="agent.setMode('settings')"
      >
        <AppIcon name="tune" />
      </button>

      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full border border-black/15 text-[#111] transition hover:bg-black/6 disabled:cursor-default disabled:opacity-40 dark:border-white/15 dark:text-zinc-200 dark:hover:bg-white/8"
        aria-label="Analyse the screen"
        title="Analyse the screen"
        :disabled="agent.isRunning.value"
        @click="agent.analyzeScreen()"
      >
        <AppIcon name="scan" />
      </button>

      <textarea
        ref="field"
        v-model="command"
        rows="1"
        :placeholder="placeholder"
        class="max-h-[152px] min-h-8 min-w-0 flex-1 resize-none bg-transparent px-1 py-1 text-body leading-6 outline-none [--scrollbar-thumb-hover:rgb(0_0_0/0.4)] [--scrollbar-thumb:rgb(0_0_0/0.22)] dark:[--scrollbar-thumb-hover:rgb(255_255_255/0.32)] dark:[--scrollbar-thumb:rgb(255_255_255/0.16)]"
        :class="
          offline && !isListening && !agent.isRunning.value
            ? 'placeholder:text-[#b3141b] dark:placeholder:text-[#ff6b72]'
            : 'placeholder:text-[#6b6b67] dark:placeholder:text-zinc-400'
        "
        @keydown.enter.exact.prevent="submit"
      />

      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full text-white transition"
        :class="
          action.tone === 'red'
            ? 'bg-[#d71921] hover:bg-[#b9151c]'
            : 'bg-[#111] hover:bg-black dark:bg-[#f1f1ef] dark:text-[#111] dark:hover:bg-white'
        "
        :aria-label="action.label"
        :aria-pressed="action.icon === 'mic' ? isListening : undefined"
        @click="action.run()"
      >
        <AppIcon :name="action.icon" />
      </button>
    </div>
  </div>
</template>