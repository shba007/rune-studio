<script setup lang="ts">
import { useFocus, useTextareaAutosize } from '@vueuse/core'
import { computed, nextTick, ref, useId, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { TaskBoardModel } from '@/composables/useTaskBoard'

import AppIcon from './AppIcon.vue'
import { screenRequestText } from '@/config/screenRequest'
import {
  composerCommands,
  exactComposerCommand,
  type ComposerCommand,
} from '@/config/composerCommands'

const props = defineProps<{
  agent: Agent
  board?: TaskBoardModel
  analysisSubmit?: (text: string) => Promise<boolean>
  analysisBusy?: boolean
  cancelAnalysis?: () => unknown
  intakeBusy?: boolean
}>()
const agent = props.agent

const { textarea: field, input: command } = useTextareaAutosize()
const { focused } = useFocus(field)
watch(
  () => agent.prefillRequest?.value,
  (request) => {
    if (request) command.value = request.text
  },
)

const hasText = computed(() => command.value.trim().length > 0)
const isListening = computed(() => agent.state.value === 'listening')
const needsConfirm = computed(() => agent.state.value === 'awaiting_confirm')
const menuId = useId()
const menuDismissed = ref(false)
const selectedCommand = ref(0)
const commandPending = ref(false)
const commandNotice = ref('')
const slash = computed(() => command.value.trimStart().startsWith('/'))
const suggestions = computed(() => composerCommands(command.value))
const showCommands = computed(() => slash.value && !menuDismissed.value && !props.analysisBusy)
watch(command, () => {
  selectedCommand.value = 0
  menuDismissed.value = false
  commandNotice.value = ''
})
function unavailableReason(item: ComposerCommand) {
  if (item.id !== 'analysis') return ''
  if (offline.value) return 'Needs the model server'
  if (!external.value && agent.isRunning.value) return 'Finish or stop the current task first'
  return ''
}
const unavailable = (item: ComposerCommand) => unavailableReason(item) !== ''
async function runCommand(item: ComposerCommand) {
  if (commandPending.value || props.analysisBusy) return
  if (unavailable(item)) {
    commandNotice.value = offline.value
      ? 'Visual Analysis needs the model server.'
      : 'Finish or stop the current task before starting Visual Analysis.'
    return
  }
  commandPending.value = true
  try {
    if (item.id === 'analysis') await agent.analyzeScreen()
    else if (item.id === 'settings') await agent.openSettings()
    else await agent.openWorkspace(item.id)
    command.value = ''
    menuDismissed.value = true
  } catch (error) {
    commandNotice.value = error instanceof Error ? error.message : String(error)
  } finally {
    commandPending.value = false
  }
}
function onKeydown(event: KeyboardEvent) {
  if (event.isComposing) return
  if (showCommands.value && event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    menuDismissed.value = true
  } else if (showCommands.value && ['ArrowDown', 'ArrowUp'].includes(event.key)) {
    event.preventDefault()
    const count = suggestions.value.length
    if (count)
      selectedCommand.value =
        (selectedCommand.value + (event.key === 'ArrowDown' ? 1 : count - 1)) % count
  } else if (showCommands.value && event.key === 'Tab' && suggestions.value.length) {
    event.preventDefault()
    command.value = suggestions.value[selectedCommand.value]!.command
  } else if (
    event.key === 'Enter' &&
    !event.shiftKey &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.metaKey
  ) {
    event.preventDefault()
    void submit()
  }
}

const action = computed(() => {
  if (props.analysisSubmit && props.analysisBusy)
    return {
      icon: 'stop',
      label: 'Cancel analysis',
      tone: 'red',
      run: () => props.cancelAnalysis?.(),
    } as const
  if (slash.value) return { icon: 'send', label: 'Run command', tone: 'dark', run: submit } as const
  if (props.analysisSubmit)
    return { icon: 'send', label: 'Analyze', tone: 'dark', run: submit } as const
  if (props.board && hasText.value)
    return { icon: 'send', label: 'Add request', tone: 'dark', run: submit } as const
  if (
    agent.isRunning.value ||
    props.intakeBusy ||
    props.board?.requests.value.some((r) => ['queued', 'planning', 'running'].includes(r.phase))
  )
    return { icon: 'stop', label: 'Stop', tone: 'red', run: () => agent.stop() } as const
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
  if (external.value) return 'Type / for commands; start tasks in your host'
  if (props.analysisSubmit) return props.analysisBusy ? 'Analyzing…' : 'Type or say a request'
  if (needsConfirm.value) return 'Needs your OK'
  if (props.board && agent.isRunning.value) return 'Add a request'
  if (agent.isRunning.value) return 'Working…'
  if (isListening.value) return 'Listening…'
  if (offline.value) return 'Offline · type / for commands'
  return 'Type or say a request'
})

watch(agent.focusTick, async () => {
  await nextTick()
  focused.value = true
})

async function submit() {
  if (commandPending.value) return
  const text = command.value.trim()
  if (slash.value) {
    const item =
      exactComposerCommand(text) ??
      (showCommands.value ? suggestions.value[selectedCommand.value] : undefined)
    if (item) await runCommand(item)
    else commandNotice.value = 'Unknown command. Type / to see available commands.'
    return
  }
  if (offline.value) return
  if (props.analysisSubmit) {
    if (!external.value && !props.analysisBusy && (await props.analysisSubmit(text)))
      command.value = ''
    return
  }
  if (!text || external.value) return
  if (props.board) {
    if (await props.board.submit(text)) {
      command.value = ''
      if (agent.mode.value === 'ambient') await agent.setMode('open')
    }
    return
  }
  if (agent.isRunning.value) return
  if (await agent.startTask(text)) command.value = ''
}
</script>

<template>
  <div
    data-tauri-drag-region
    class="flex w-full flex-col gap-rune-detail rounded-rune-panel bg-rune-surface p-rune-1 font-rune-reading"
  >
    <p
      v-if="agent.notice.value"
      class="m-0 px-rune-1 pt-rune-detail text-rune-caption"
      :class="agent.noticeKind.value === 'info' ? 'text-zinc-200' : 'text-error'"
      role="status"
      aria-live="polite"
    >
      {{ agent.notice.value }}
    </p>
    <p
      v-if="external"
      class="m-0 px-rune-1 pt-rune-detail text-rune-caption text-zinc-400"
      role="status"
    >
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
    <p v-if="board?.notice.value" role="alert" class="m-0 px-rune-1 text-rune-caption text-error">
      {{ board.notice.value }}
    </p>
    <p class="sr-only" role="status" aria-live="polite">{{ agent.statusText.value }}</p>
    <p v-if="commandNotice" role="alert" class="m-0 px-rune-1 text-rune-caption text-zinc-400">
      {{ commandNotice }}
    </p>
    <div
      v-if="showCommands"
      :id="menuId"
      role="listbox"
      aria-label="Composer commands"
      class="rounded-rune-control border border-rune-outline bg-rune-raised p-rune-detail text-zinc-100"
    >
      <button
        v-for="(item, index) in suggestions"
        :id="`${menuId}-${index}`"
        :key="item.id"
        role="option"
        :aria-selected="index === selectedCommand"
        type="button"
        :disabled="unavailable(item) || commandPending"
        class="flex w-full cursor-pointer items-center gap-rune-1 rounded-xl px-rune-1 py-rune-1 text-left text-rune-body text-zinc-100 disabled:cursor-default disabled:opacity-40"
        :class="index === selectedCommand ? 'bg-white/10' : 'hover:bg-white/5'"
        @mousedown.prevent
        @click="runCommand(item)"
      >
        <AppIcon :name="item.icon" size="sm" /><span
          ><span class="block">{{ item.command }}</span
          ><span class="text-rune-caption text-zinc-400">{{
            unavailableReason(item) || item.description
          }}</span></span
        >
      </button>
      <p v-if="!suggestions.length" class="m-0 px-rune-1 py-rune-1 text-rune-body text-zinc-400">
        No matching commands
      </p>
    </div>
    <div
      class="flex w-full items-end gap-rune-1 rounded-rune-control bg-white/5 p-rune-1 text-zinc-100 focus-within:bg-white/8 transition-colors"
    >
      <textarea
        ref="field"
        v-model="command"
        rows="1"
        :disabled="
          (offline && Boolean(analysisSubmit)) ||
          commandPending ||
          (Boolean(analysisSubmit) && analysisBusy)
        "
        aria-label="Request or slash command"
        :aria-expanded="showCommands"
        :aria-controls="showCommands ? menuId : undefined"
        :aria-activedescendant="
          showCommands && suggestions.length ? `${menuId}-${selectedCommand}` : undefined
        "
        :placeholder="placeholder"
        class="text-zinc-100 placeholder:text-zinc-400 max-h-[152px] min-h-8 min-w-0 flex-1 resize-none bg-transparent px-rune-detail py-rune-detail text-rune-body leading-6 outline-none [--scrollbar-thumb-hover:rgb(0_0_0/0.4)] [--scrollbar-thumb:rgb(0_0_0/0.22)] dark:[--scrollbar-thumb-hover:rgb(255_255_255/0.32)] dark:[--scrollbar-thumb:rgb(255_255_255/0.16)]"
        @keydown="onKeydown"
      />

      <button
        type="button"
        class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-full transition"
        :class="
          action.tone === 'red'
            ? 'bg-attention text-white hover:bg-attention-hover'
            : action.tone === 'blue'
              ? 'bg-voice hover:bg-voice-hover'
              : 'bg-primary !text-[#111] hover:bg-white'
        "
        :disabled="
          commandPending ||
          (external && !slash && action.icon !== 'stop') ||
          (!slash && !analysisSubmit && board?.pending.value) ||
          (offline && !slash && action.icon !== 'stop')
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