<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'
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
        :info="
          draft.externalAgent
            ? 'Start tasks and visual analysis in your external agent host. Studio provides capture and input tools without calling its API model.'
            : 'Studio runs tasks and visual analysis using the configured API models.'
        "
      >
        <RuneChoice
          v-model="draft.externalAgent"
          label="Execution mode"
          :disabled="agent.isRunning.value"
          :options="[
            { value: false, label: 'Studio model' },
            { value: true, label: 'External agent' },
          ]"
        />
      </HubRow>
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