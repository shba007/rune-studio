<script setup lang="ts">
import RuneChoice from './RuneChoice.vue'
import { ref } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import DevInputLab from './DevInputLab.vue'
import HubGroup from './HubGroup.vue'
import HubToggle from './HubToggle.vue'
import HubRow from './HubRow.vue'
import SettingsReview from './SettingsReview.vue'
import RecordingControls from './RecordingControls.vue'
import SettingsGoalContext from './SettingsGoalContext.vue'

withDefaults(
  defineProps<{
    agent: Agent
    draft: Settings
    view?: 'all' | 'general' | 'models' | 'capture' | 'developer'
  }>(),
  { view: 'all' },
)

const MODEL_RESOLUTIONS = [
  { value: '720', label: '720 px · Faster' },
  { value: '1280', label: '1280 px · Balanced' },
  { value: '1920', label: '1920 px · Detailed' },
  { value: '2560', label: '2560 px · More detailed' },
  { value: 'native', label: 'Native resolution' },
]
const RECORDING_RESOLUTIONS = [
  { value: '720', label: '720 px' },
  { value: '1280', label: '1280 px' },
  { value: '1920', label: '1920 px' },
  { value: '2560', label: '2560 px' },
  { value: 'native', label: 'Native resolution' },
]

const DEBUG_STATES: AgentState[] = [
  'idle',
  'listening',
  'planning',
  'acting',
  'awaiting_confirm',
  'done',
  'failed',
]

const previewState = ref<AgentState>('idle')
const PREVIEW_STATES = DEBUG_STATES.map((value) => ({ value, label: value.replace('_', ' ') }))
</script>

<template>
  <div class="flex flex-col gap-8">
    <SettingsGoalContext v-if="view === 'all'" :draft="draft" />
    <HubGroup v-if="view === 'all' || view === 'models'" title="Model input">
      <HubRow
        label="Model input resolution"
        hint="Limits images sent to models for faster inference. Independent of saved screenshot and recording quality."
      >
        <RuneChoice
          v-model="draft.modelImageResolution"
          label="Model input resolution"
          :options="MODEL_RESOLUTIONS"
        />
      </HubRow>
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'capture'" title="Recording">
      <SettingsReview :draft="draft" />
      <HubToggle
        v-model="draft.demoMode"
        label="Show Rune in screen recordings"
        hint="OBS, Game Bar and the Snipping Tool can capture Rune's windows."
        info="Rune hides itself from its own model either way."
      />
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'capture'" title="Capture quality">
      <HubRow
        label="Screenshot and recording resolution"
        hint="Quality of new saved screenshots and recordings."
        info="Limits the longest edge without enlarging captures. Native preserves captured dimensions. Model input resolution is independent; video remains silent at 2 fps."
      >
        <RuneChoice
          v-model="draft.recordingResolution"
          label="Screenshot and recording resolution"
          :options="RECORDING_RESOLUTIONS"
        />
      </HubRow>
    </HubGroup>

    <RecordingControls v-if="view === 'all' || view === 'capture'" />

    <HubGroup v-if="view === 'all' || view === 'general'" title="Safety &amp; behavior">
      <HubToggle
        v-model="draft.askEveryAction"
        label="Ask before every action"
        hint="Request approval for each action, in addition to existing safety checks."
      /><HubToggle
        v-if="view === 'general'"
        v-model="draft.showThinking"
        label="Show thinking"
        hint="Show model thinking in the orb bubble when available."
      />
    </HubGroup>

    <HubGroup v-if="view === 'all' || view === 'developer'" title="Developer tools">
      <HubToggle
        v-model="draft.developerMode"
        label="Developer mode"
        hint="Show screen capture, input tests, animation previews and raw task records."
      />
    </HubGroup>

    <template v-if="(view === 'all' || view === 'developer') && draft.developerMode">
      <HubGroup title="Animation states">
        <HubRow label="Preview state" hint="Force the orb into a state to check its animation.">
          <RuneChoice
            v-model="previewState"
            label="Preview state"
            :options="PREVIEW_STATES"
            menu
            @update:model-value="agent.debugSetState($event)"
          />
        </HubRow>
      </HubGroup>
      <DevInputLab :agent="agent" />
    </template>
  </div>
</template>