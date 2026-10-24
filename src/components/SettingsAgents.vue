<script setup lang="ts">
import { useClipboard } from '@vueuse/core'

import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import SettingsConnectionModels from './SettingsConnectionModels.vue'

defineProps<{ agent: Agent; draft: Settings }>()

const apiKey = defineModel<string>('apiKey', { default: '' })
defineEmits<{ 'commit-key': [] }>()

const { copy, copied } = useClipboard({ copiedDuring: 1600 })
</script>

<template>
  <div class="flex flex-col gap-8">
    <SettingsConnectionModels
      :agent="agent"
      :draft="draft"
      v-model:api-key="apiKey"
      @commit-key="$emit('commit-key')"
    />
    <HubGroup title="Execution">
      <HubRow
        label="Execution mode"
        hint="Choose before a session. Finish or stop it before switching."
      >
        <select
          v-model="draft.externalAgent"
          :disabled="agent.isRunning.value"
          aria-label="Execution mode"
          class="rounded-lg bg-zinc-800 px-3 py-2 text-label"
        >
          <option :value="false">API — Studio model</option>
          <option :value="true">External agent — host model</option>
        </select>
      </HubRow>
      <p class="m-0 text-label text-zinc-500">
        {{
          draft.externalAgent
            ? 'Start tasks and screen analysis in your external agent host. Studio provides capture and input tools without calling its API model.'
            : 'Studio runs tasks and screen analysis using the configured API models.'
        }}
      </p>
      <HubRow
        v-if="draft.externalAgent && agent.bridge.value?.available"
        label="Connect command"
        :hint="agent.bridge.value.url"
      >
        <button
          type="button"
          class="flex h-9 cursor-pointer items-center gap-2 rounded-full border border-white/25 bg-transparent px-4 text-label text-zinc-200 uppercase transition hover:bg-white/10"
          @click="copy(agent.bridge.value.command)"
        >
          <AppIcon name="copy" size="sm" />
          {{ copied ? 'Copied' : 'Copy' }}
        </button>
      </HubRow>
    </HubGroup>
  </div>
</template>