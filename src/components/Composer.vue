<script setup lang="ts">
import { useFocus, useTextareaAutosize } from '@vueuse/core'
import { computed, nextTick, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

import AppIcon from './AppIcon.vue'
import { screenRequestText } from '@/config/screenRequest'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const { textarea: field, input: command } = useTextareaAutosize()
const { focused } = useFocus(field)

const hasText = computed(() => command.value.trim().length > 0)
const isListening = computed(() => agent.state.value === 'listening')
const needsConfirm = computed(() => agent.state.value === 'awaiting_confirm')

const action = computed(() => {
  if (agent.isRunning.value)
    return { icon: 'stop', label: 'Stop', tone: 'blue', run: () => agent.stop() } as const
  if (hasText.value) return { icon: 'send', label: 'Send', tone: 'dark', run: submit } as const
  return {
    icon: 'mic',
    label: isListening.value ? 'Stop listening' : 'Listen',
    tone: 'blue',
    run: () => agent.toggleListening(),
  } as const
})

const external = computed(() => agent.settings.value?.externalAgent === true)
const offline = computed(() => agent.health.value.tone === 'down')
const placeholder = computed(() => {
  if (external.value) return 'Start tasks in your external agent host'
  if (needsConfirm.value) return 'Needs your OK'
  if (agent.isRunning.value) return 'Working…'
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
  if (!text || agent.isRunning.value || external.value) return
  if (await agent.startTask(text)) command.value = ''
}
</script>

<template>
  <div data-tauri-drag-region class="flex w-full flex-col gap-1 rounded-[28px] bg-[#141414] p-1.5">
    <p
      v-if="agent.notice.value"
      class="m-0 px-3 pt-1 text-label"
      :class="agent.noticeKind.value === 'info' ? 'text-zinc-200' : 'text-[#ff6b72]'"
      role="status"
      aria-live="polite"
    >
      {{ agent.notice.value }}
    </p>
    <p v-if="external" class="m-0 px-3 pt-1 text-label text-zinc-400" role="status">
      {{ screenRequestText(agent.screenRequest.value) }}
      <button
        v-if="
          agent.screenRequest.value.waiting ||
          ['queued', 'delivered'].includes(agent.screenRequest.value.status)
        "
        type="button"
        class="ml-2 cursor-pointer underline"
        @click="agent.cancelScreenRequest()"
      >
        Cancel screen request
      </button>
    </p>
    <p class="sr-only" role="status" aria-live="polite">{{ agent.statusText.value }}</p>
    <div
      class="flex w-full items-end gap-2 rounded-[22px] bg-white/5 p-2 text-zinc-100 focus-within:bg-white/8 transition-colors"
    >
      <textarea
        ref="field"
        v-model="command"
        rows="1"
        :disabled="external"
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
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full transition"
        :class="
          action.tone === 'blue'
            ? 'bg-[#2d71c3] hover:bg-[#245fa8]'
            : 'bg-[#f1f1ef] !text-[#111] hover:bg-white'
        "
        :disabled="external && !agent.isRunning.value"
        :aria-label="action.label"
        :aria-pressed="action.icon === 'mic' ? isListening : undefined"
        @click="action.run()"
      >
        <AppIcon :name="action.icon" />
      </button>
    </div>
  </div>
</template>