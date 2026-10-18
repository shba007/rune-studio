<script setup lang="ts">
import { useFocus } from '@vueuse/core'
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'

import AppIcon from './AppIcon.vue'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const command = ref('')
const field = useTemplateRef<HTMLTextAreaElement>('field')
const { focused } = useFocus(field)

const canSend = computed(() => command.value.trim().length > 0 && !agent.isRunning.value)
const isListening = computed(() => agent.state.value === 'listening')
const needsConfirm = computed(() => agent.state.value === 'awaiting_confirm')

/** The backend chip is now a dot: green when fine, amber or red otherwise, details on hover. */
const backend = computed(() => {
  const status = agent.backend.value
  const model = agent.settings.value?.visionModel ?? ''
  if (!status) return { dot: 'bg-zinc-500', title: 'Checking the backend' }
  if (!status.reachable) return { dot: 'bg-[#d71921]', title: status.error ?? 'Backend offline' }
  if (!status.authOk) return { dot: 'bg-amber-400', title: 'API key rejected' }
  if (status.missingModels.length > 0) {
    return { dot: 'bg-amber-400', title: `Missing model: ${status.missingModels.join(', ')}` }
  }
  if (status.error) return { dot: 'bg-amber-400', title: status.error }
  return { dot: 'bg-emerald-400', title: `Backend OK, ${status.latencyMs ?? '?'} ms, ${model}` }
})

watch(agent.focusTick, async () => {
  await nextTick()
  focused.value = true
})

async function submit() {
  const text = command.value.trim()
  if (!canSend.value) return
  if (await agent.startTask(text)) command.value = ''
}
</script>

<template>
  <div
    data-tauri-drag-region
    class="flex w-full flex-col rounded-[28px] border border-white/8 bg-[#141414] p-2"
  >
    <div class="flex flex-col rounded-[20px] bg-[#f1f1ef] px-4 pt-4 pb-4 text-[#111]">
      <textarea
        ref="field"
        v-model="command"
        rows="3"
        placeholder="Type or say a command"
        class="h-14 w-full resize-none bg-transparent text-body outline-none placeholder:text-[#8a8a86] [--scrollbar-thumb-hover:rgb(0_0_0/0.4)] [--scrollbar-thumb:rgb(0_0_0/0.22)]"
        @keydown.enter.exact.prevent="submit"
      />

      <div class="flex items-center gap-2 pt-1">
        <button
          type="button"
          class="grid size-8 cursor-pointer place-items-center rounded-full border border-black/15 text-[#111] transition hover:bg-black/6"
          aria-label="Settings"
          @click="agent.setMode('settings')"
        >
          <AppIcon name="tune" />
        </button>

        <!-- Mic and send share one look: same size, shape and fill. Only their state differs. -->
        <div class="ml-auto flex items-center gap-2">
          <button
            type="button"
            class="grid size-8 cursor-pointer place-items-center rounded-full text-white transition"
            :class="isListening ? 'bg-[#d71921]' : 'bg-[#111] hover:bg-black'"
            :aria-label="isListening ? 'Stop listening' : 'Listen'"
            :aria-pressed="isListening"
            @click="agent.toggleListening()"
          >
            <AppIcon name="mic" />
          </button>

          <button
            type="button"
            class="grid size-8 cursor-pointer place-items-center rounded-full text-white transition"
            :class="
              agent.isRunning.value
                ? 'bg-[#d71921] hover:bg-[#b9151c]'
                : canSend
                  ? 'bg-[#111] opacity-100 hover:bg-black'
                  : 'bg-[#111] opacity-35'
            "
            :aria-label="agent.isRunning.value ? 'Stop' : 'Send'"
            @click="agent.isRunning.value ? agent.stop() : submit()"
          >
            <AppIcon :name="agent.isRunning.value ? 'stop' : 'send'" />
          </button>
        </div>
      </div>
    </div>

    <div class="flex min-h-10 items-center gap-2 px-2">
      <span class="size-2 shrink-0 rounded-full" :class="backend.dot" :title="backend.title" />

      <span
        class="line-clamp-2 min-w-0 flex-1 text-status"
        :class="
          agent.notice.value ? 'text-amber-300' : needsConfirm ? 'text-[#d71921]' : 'text-zinc-300'
        "
        :title="agent.statusText.value"
      >
        {{ agent.statusText.value }}
      </span>

      <template v-if="needsConfirm">
        <button
          type="button"
          class="cursor-pointer rounded-full border border-[#d71921] h-8 px-3 text-label text-[#d71921] hover:bg-[#d71921]/15"
          @click="agent.confirm('pending', true)"
        >
          Yes
        </button>
        <button
          type="button"
          class="cursor-pointer rounded-full border border-zinc-600 h-8 px-3 text-label text-zinc-400 hover:bg-white/10"
          @click="agent.confirm('pending', false)"
        >
          No
        </button>
      </template>
    </div>
  </div>
</template>