<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { useClipboard } from '@vueuse/core'

import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import SettingsVisualAnalysis from './SettingsVisualAnalysis.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import SettingsConnectionModels from './SettingsConnectionModels.vue'

withDefaults(defineProps<{ agent: Agent; draft: Settings; executionOnly?: boolean }>(), {
  executionOnly: false,
})

const { copy, copied } = useClipboard({ copiedDuring: 1600 })
</script>

<template>
  <div class="flex flex-col gap-8">
    <SettingsConnectionModels v-if="!executionOnly" :agent="agent" :draft="draft" />
    <SettingsVisualAnalysis v-if="!executionOnly" :draft="draft" />
    <HubGroup title="Execution">
      <HubRow
        label="Execution mode"
        hint="Choose before a task. Finish or stop it before switching."
      >
        <select
          v-model="draft.externalAgent"
          :disabled="agent.isRunning.value"
          aria-label="Execution mode"
          class="rounded-rune-tile bg-rune-raised px-rune-2 py-2 text-rune-body"
        >
          <option :value="false">API — Studio model</option>
          <option :value="true">External agent — host model</option>
        </select>
      </HubRow>
      <p class="m-0 text-rune-body text-rune-text-2">
        {{
          draft.externalAgent
            ? 'Start tasks and visual analysis in your external agent host. Studio provides capture and input tools without calling its API model.'
            : 'Studio runs tasks and visual analysis using the configured API models.'
        }}
      </p>
      <HubRow
        v-if="draft.externalAgent && agent.bridge.value?.available"
        label="Connect command"
        :hint="agent.bridge.value.url"
      >
        <RuneButton variant="ghost" size="sm" @click="copy(agent.bridge.value.command)">
          <AppIcon name="copy" size="sm" />
          {{ copied ? 'Copied' : 'Copy' }}
        </RuneButton>
      </HubRow>
    </HubGroup>
  </div>
</template>